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

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use data_model_ltx::{
    models::{JobIdPayload, JobIdResponse, JobKind, JobStatus, LlmTxtResponse, LlmsTxtListResponse, UrlPayload},
    test_helpers::{clean_test_db, create_completed_test_job, create_test_job, test_db_pool},
};
use http_body_util::BodyExt;
use tokio::sync::Mutex;
use tower::ServiceExt;

use api_ltx::routes::router;

// =============================================================================
// Test Database Lifecycle Management (Cross-Process Singleton)
// =============================================================================
//
// Uses file-based coordination to manage the test database lifecycle across
// multiple test binaries (e.g., when running `cargo test --workspace`).
//
// Files used (in system temp directory):
// - llm-web-index-test-db.lock: File lock for atomic operations
// - llm-web-index-test-db.state: Stores "count:started_externally" (e.g., "3:false")
//
// Protocol:
// - On acquire: lock, read state, if count==0 && DB not running -> start DB,
//   increment count, write state, unlock
// - On release: lock, read state, decrement count, if count==0 && we started it
//   -> stop DB, unlock

const LOCK_FILE_NAME: &str = "llm-web-index-test-db.lock";
const STATE_FILE_NAME: &str = "llm-web-index-test-db.state";

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

fn workspace_root() -> &'static std::path::Path {
    // api-ltx is at src/api-ltx/, so workspace root is ../../
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn compose_file() -> PathBuf {
    workspace_root().join("docker-compose.test.yml")
}

fn setup_script() -> PathBuf {
    workspace_root().join("scripts/setup_test_db.sh")
}

/// Acquire an exclusive lock on the lock file.
fn lock_exclusive(file: &File) {
    unsafe {
        if libc::flock(file.as_raw_fd(), libc::LOCK_EX) != 0 {
            panic!("Failed to acquire file lock");
        }
    }
}

/// Release the lock on the lock file.
fn unlock(file: &File) {
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
}

/// State stored in the state file: (active_count, db_was_already_running)
#[derive(Debug, Clone, Copy)]
struct DbState {
    count: usize,
    was_already_running: bool,
}

impl DbState {
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split(':').collect();
        if parts.len() == 2 {
            let count = parts[0].parse().ok()?;
            let was_already_running = parts[1].parse().ok()?;
            Some(Self {
                count,
                was_already_running,
            })
        } else {
            None
        }
    }

    fn serialize(&self) -> String {
        format!("{}:{}", self.count, self.was_already_running)
    }
}

fn read_state(file: &mut File) -> DbState {
    let mut contents = String::new();
    file.seek(SeekFrom::Start(0)).ok();
    file.read_to_string(&mut contents).ok();
    DbState::parse(&contents).unwrap_or(DbState {
        count: 0,
        was_already_running: false,
    })
}

fn write_state(file: &mut File, state: &DbState) {
    file.seek(SeekFrom::Start(0)).unwrap();
    file.set_len(0).unwrap();
    file.write_all(state.serialize().as_bytes()).unwrap();
    file.sync_all().unwrap();
}

fn is_db_running() -> bool {
    let check_cmd = format!(
        "docker compose -f {} ps postgres-test 2>/dev/null | grep -q Up",
        compose_file().display()
    );
    Command::new("sh")
        .arg("-c")
        .arg(&check_cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_db_healthy() -> bool {
    let health_cmd = format!(
        "docker compose -f {} ps postgres-test 2>/dev/null | grep -q healthy",
        compose_file().display()
    );
    Command::new("sh")
        .arg("-c")
        .arg(&health_cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn start_db() {
    eprintln!("[TestDbGuard] Starting test database via setup_test_db.sh...");
    let status = Command::new(setup_script())
        .current_dir(workspace_root())
        .status()
        .expect("Failed to run setup_test_db.sh");
    if !status.success() {
        panic!("Failed to start test database");
    }

    // Wait for database to be healthy
    eprintln!("[TestDbGuard] Waiting for test database to be healthy...");
    let max_attempts = 30;
    for attempt in 1..=max_attempts {
        if is_db_healthy() {
            eprintln!("[TestDbGuard] Test database is healthy.");
            return;
        }
        eprintln!("[TestDbGuard]   Attempt {}/{}: waiting...", attempt, max_attempts);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    panic!("Test database failed to become healthy after {} attempts", max_attempts);
}

fn stop_db() {
    eprintln!("[TestDbGuard] Shutting down test database...");
    let _ = Command::new("docker")
        .args(["compose", "-f", compose_file().to_str().unwrap(), "down"])
        .status();
}

/// Guard that represents a test's usage of the test database.
/// Uses file-based locking for cross-process coordination.
pub struct TestDbGuard {
    _private: (), // prevent construction outside of acquire()
}

impl TestDbGuard {
    /// Acquire a guard for test database usage.
    /// Coordinates with other processes via file locking.
    pub async fn acquire() -> Self {
        // Open/create lock file
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(temp_path(LOCK_FILE_NAME))
            .expect("Failed to open lock file");

        // Open/create state file
        let mut state_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(temp_path(STATE_FILE_NAME))
            .expect("Failed to open state file");

        // Acquire exclusive lock
        lock_exclusive(&lock_file);

        // Read current state
        let mut state = read_state(&mut state_file);

        if state.count == 0 {
            // We're the first acquirer in this session
            let db_running = is_db_running();
            state.was_already_running = db_running;

            if !db_running {
                start_db();
            } else {
                eprintln!("[TestDbGuard] Test database already running (started externally).");
            }
        }

        // Increment count
        state.count += 1;
        eprintln!("[TestDbGuard] Acquired (active count: {})", state.count);
        write_state(&mut state_file, &state);

        // Release lock
        unlock(&lock_file);

        Self { _private: () }
    }
}

impl Drop for TestDbGuard {
    fn drop(&mut self) {
        // Open lock file
        let lock_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(temp_path(LOCK_FILE_NAME))
        {
            Ok(f) => f,
            Err(_) => return, // Lock file gone, nothing to do
        };

        // Open state file
        let mut state_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(temp_path(STATE_FILE_NAME))
        {
            Ok(f) => f,
            Err(_) => return, // State file gone, nothing to do
        };

        // Acquire exclusive lock
        lock_exclusive(&lock_file);

        // Read and decrement count
        let mut state = read_state(&mut state_file);
        state.count = state.count.saturating_sub(1);
        eprintln!("[TestDbGuard] Released (active count: {})", state.count);

        if state.count == 0 {
            if state.was_already_running {
                eprintln!("[TestDbGuard] Last user done. DB was started externally, leaving it running.");
            } else {
                eprintln!("[TestDbGuard] Last user done. Shutting down test database.");
                stop_db();
            }
            // Clean up state file for next session
            let _ = std::fs::remove_file(temp_path(STATE_FILE_NAME));
        } else {
            write_state(&mut state_file, &state);
        }

        // Release lock
        unlock(&lock_file);
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
