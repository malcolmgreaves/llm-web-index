//! Integration tests for API route handlers
//!
//! Tests key endpoints:
//! - GET /api/llm_txt - Retrieve llms.txt content
//! - POST /api/llm_txt - Create generation job
//! - POST /api/update - Create update job
//! - PUT /api/llm_txt - Create job (new or update)
//! - GET /api/list - List all llms.txt
//! - POST /api/status - Get job status
//! - GET /api/job - Get job details
//! - GET /api/jobs/in_progress - List in-progress jobs

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use data_model_ltx::{
    models::{JobIdPayload, JobIdResponse, JobKind, JobStatus, LlmTxtResponse, LlmsTxtListResponse, UrlPayload},
    test_helpers::{clean_test_db, create_completed_test_job, create_test_job, test_db_pool},
};
use http_body_util::BodyExt;
use tokio::sync::{Mutex, OnceCell};
use tower::ServiceExt;

use api_ltx::routes::router;

// =============================================================================
// Test Database Lifecycle Management
// =============================================================================

/// Shared state for test database lifecycle management.
/// Tracks the number of active test users and handles DB startup/shutdown.
struct TestDbState {
    active_count: AtomicUsize,
}

/// Lazily-initialized shared state for the test database.
static TEST_DB_STATE: OnceCell<Arc<TestDbState>> = OnceCell::const_new();

/// Guard that represents a test's usage of the test database.
/// Increments counter on creation, decrements on drop.
/// Shuts down DB when last guard is dropped.
pub struct TestDbGuard {
    state: Arc<TestDbState>,
}

impl TestDbGuard {
    /// Acquire a guard for test database usage.
    /// On first call, checks if DB is running and starts it if needed.
    pub async fn acquire() -> Self {
        // Get workspace root from CARGO_MANIFEST_DIR (api-ltx is at src/api-ltx/)
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let compose_file = workspace_root.join("docker-compose.test.yml");
        let setup_script = workspace_root.join("scripts/setup_test_db.sh");

        let state = TEST_DB_STATE
            .get_or_init(|| async {
                // Check if DB is running
                let check_cmd = format!(
                    "docker compose -f {} ps postgres-test 2>/dev/null | grep -q Up",
                    compose_file.display()
                );
                let is_running = Command::new("sh")
                    .arg("-c")
                    .arg(&check_cmd)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                if !is_running {
                    eprintln!("Test database not running, starting via setup_test_db.sh...");
                    let status = Command::new(&setup_script)
                        .current_dir(&workspace_root)
                        .status()
                        .expect("Failed to run setup_test_db.sh");
                    if !status.success() {
                        panic!("Failed to start test database");
                    }
                }

                Arc::new(TestDbState {
                    active_count: AtomicUsize::new(0),
                })
            })
            .await
            .clone();

        state.active_count.fetch_add(1, Ordering::SeqCst);
        Self { state }
    }
}

impl Drop for TestDbGuard {
    fn drop(&mut self) {
        let prev = self.state.active_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            // We were the last user, shut down the database
            let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap();
            let compose_file = workspace_root.join("docker-compose.test.yml");

            eprintln!("Last test finished, shutting down test database...");
            let _ = Command::new("docker")
                .args(["compose", "-f", compose_file.to_str().unwrap(), "down"])
                .status();
        }
    }
}

// =============================================================================

/// Helper to create a router with test database (does NOT clean DB)
async fn test_router() -> axum::Router {
    let pool = test_db_pool().await;
    router(None).with_state(pool)
}

/// Helper to parse JSON response body
async fn response_json<T: serde::de::DeserializeOwned>(body: Body) -> T {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// These tests require sequential execution.
static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

// NOTE: This enables detailed logging in the api-ltx service.
//       Uncomment for debugging.
#[allow(unused)]
fn debug_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "api_ltx=trace,axum::rejection=trace,tower_http=trace,hyper=trace",
        ))
        .with_test_writer()
        .try_init()
        .ok();
}

//
// GET /api/llm_txt tests
//

#[tokio::test]
async fn test_get_llm_txt_success() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;
    // debug_logging();

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create completed job
    let url = "https://example.com";
    let content = "# Test\n\n> Description\n\n- [Link](/)";
    create_completed_test_job(&pool, url, content, "<html></html>").await;

    let app = test_router().await;

    let request = Request::builder()
        .uri(format!("/api/llm_txt?url={}", urlencoding::encode(url)))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: LlmTxtResponse = response_json(response.into_body()).await;
    assert_eq!(body.content, content);
}

#[tokio::test]
async fn test_get_llm_txt_not_found() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let app = test_router().await;

    let request = Request::builder()
        .uri("/api/llm_txt?url=https://nonexistent.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.status().is_client_error() || response.status().is_server_error());
}

//
// POST /api/llm_txt tests
//

#[tokio::test]
async fn test_post_llm_txt_creates_job() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let app = test_router().await;

    let payload = UrlPayload {
        url: "https://unique-test-url.com".to_string(),
    };

    let request = Request::builder()
        .method("POST")
        .uri("/api/llm_txt")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // TODO: Known bug - in_progress_jobs returns Ok([]) instead of Err(NotFound), causing 409 even for new URLs
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: JobIdResponse = response_json(response.into_body()).await;
    assert!(!body.job_id.is_nil());
}

#[tokio::test]
async fn test_post_llm_txt_fails_if_already_generated() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let url = "https://example.com";
    create_completed_test_job(&pool, url, "# Existing", "<html></html>").await;

    let app = test_router().await;

    let payload = UrlPayload { url: url.to_string() };

    let request = Request::builder()
        .method("POST")
        .uri("/api/llm_txt")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.status().is_client_error());
}

//
// POST /api/update tests
//

#[tokio::test]
async fn test_post_update_creates_job() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let url = "https://example.com";
    create_completed_test_job(&pool, url, "# Existing", "<html></html>").await;

    let app = test_router().await;

    let payload = UrlPayload { url: url.to_string() };

    let request = Request::builder()
        .method("POST")
        .uri("/api/update")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: JobIdResponse = response_json(response.into_body()).await;
    assert!(!body.job_id.is_nil());
}

//
// PUT /api/llm_txt tests
//

#[tokio::test]
async fn test_put_llm_txt_creates_new_job() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let app = test_router().await;

    let payload = UrlPayload {
        url: "https://newsite.com".to_string(),
    };

    let request = Request::builder()
        .method("PUT")
        .uri("/api/llm_txt")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: JobIdResponse = response_json(response.into_body()).await;
    assert!(!body.job_id.is_nil());
}

#[tokio::test]
async fn test_put_llm_txt_creates_update_job_when_exists() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let url = "https://example.com";
    create_completed_test_job(&pool, url, "# Existing", "<html></html>").await;

    let app = test_router().await;

    let payload = UrlPayload { url: url.to_string() };

    let request = Request::builder()
        .method("PUT")
        .uri("/api/llm_txt")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

//
// GET /api/list tests
//

#[tokio::test]
async fn test_get_list_empty() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let app = test_router().await;

    let request = Request::builder().uri("/api/list").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: LlmsTxtListResponse = response_json(response.into_body()).await;
    assert_eq!(body.items.len(), 0);
}

#[tokio::test]
async fn test_get_list_returns_results() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    create_completed_test_job(&pool, "https://site1.com", "# Site 1", "<html>1</html>").await;
    create_completed_test_job(&pool, "https://site2.com", "# Site 2", "<html>2</html>").await;
    create_completed_test_job(&pool, "https://site3.com", "# Site 3", "<html>3</html>").await;

    let app = test_router().await;

    let request = Request::builder().uri("/api/list").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: LlmsTxtListResponse = response_json(response.into_body()).await;
    assert_eq!(body.items.len(), 3);
}

//
// POST /api/status tests
//

#[tokio::test]
async fn test_get_status_success() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Queued).await;

    let app = test_router().await;

    let payload = JobIdPayload { job_id: job.job_id };

    // Note: GET with JSON body is unusual but that's how the API is designed
    let request = Request::builder()
        .method("GET")
        .uri("/api/status")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

//
// GET /api/job tests
//

#[tokio::test]
async fn test_get_job_success() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Queued).await;

    let app = test_router().await;

    let request = Request::builder()
        .uri(format!("/api/job?job_id={}", job.job_id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

//
// GET /api/jobs/in_progress tests
//

#[tokio::test]
async fn test_get_in_progress_jobs_empty() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    let app = test_router().await;

    let request = Request::builder()
        .uri("/api/jobs/in_progress")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Vec<data_model_ltx::models::JobState> = response_json(response.into_body()).await;
    assert_eq!(body.len(), 0);
}

#[tokio::test]
async fn test_get_in_progress_jobs_returns_queued() {
    let _db = TestDbGuard::acquire().await;
    let _guard = TEST_MUTEX.lock().await;

    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    create_test_job(&pool, "https://site1.com", JobKind::New, JobStatus::Queued).await;
    create_test_job(&pool, "https://site2.com", JobKind::New, JobStatus::Queued).await;

    let app = test_router().await;

    let request = Request::builder()
        .uri("/api/jobs/in_progress")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Vec<data_model_ltx::models::JobState> = response_json(response.into_body()).await;
    assert_eq!(body.len(), 2);
}
