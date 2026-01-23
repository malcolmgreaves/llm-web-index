//! Test utilities for database operations
//!
//! This module provides helpers for setting up and managing test databases,
//! creating test data, and cleaning up after tests.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::Command;

use crate::db::{DbPool, establish_connection_pool};
use crate::models::{JobKind, JobKindData, JobState, JobStatus, LlmsTxt, LlmsTxtResult};
use crate::schema;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

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

/// Returns the workspace root directory.
/// This function looks for the Cargo.toml with [workspace] to find the root.
fn workspace_root() -> PathBuf {
    // Start from the CARGO_MANIFEST_DIR of this crate (data-model-ltx)
    // which is at src/data-model-ltx/, so workspace root is ../../
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string());
    std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn compose_file() -> PathBuf {
    workspace_root().join("docker-compose.test.yml")
}

fn setup_script() -> PathBuf {
    workspace_root().join("scripts/setup_test_db.sh")
}

/// Acquire an exclusive lock on the lock file.
#[cfg(unix)]
fn lock_exclusive(file: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        if libc::flock(file.as_raw_fd(), libc::LOCK_EX) != 0 {
            panic!("Failed to acquire file lock");
        }
    }
}

/// Release the lock on the lock file.
#[cfg(unix)]
fn unlock(file: &File) {
    use std::os::unix::io::AsRawFd;
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
///
/// # Example
/// ```ignore
/// #[tokio::test]
/// async fn my_test() {
///     let _db = TestDbGuard::acquire().await;
///     let pool = test_db_pool().await;
///     // ... test code
/// }
/// ```
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
// Database Test Helpers
// =============================================================================

/// Get a connection pool for the test database
///
/// Uses the TEST_DATABASE_URL environment variable, or falls back to a default
/// test database URL if not set.
pub async fn test_db_pool() -> DbPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db".to_string());

    establish_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool - is the test database running?")
}

/// Clean all data from the test database
///
/// Truncates both the job_state and llms_txt tables to ensure a clean slate for tests.
/// This should be called at the beginning of tests that need an empty database.
pub async fn clean_test_db(pool: &DbPool) {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    // Delete in order to respect foreign key constraints
    diesel::delete(schema::llms_txt::table)
        .execute(&mut conn)
        .await
        .expect("Failed to clean llms_txt table");

    diesel::delete(schema::job_state::table)
        .execute(&mut conn)
        .await
        .expect("Failed to clean job_state table");
}

/// Create a test job in the database
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `url` - URL for the job
/// * `kind` - Type of job (New or Update)
/// * `status` - Initial status of the job
///
/// # Returns
/// The created JobState with its generated UUID
pub async fn create_test_job(pool: &DbPool, url: &str, kind: JobKind, status: JobStatus) -> JobState {
    create_test_job_with_kind_data(
        pool,
        url,
        match kind {
            JobKind::New => JobKindData::New,
            JobKind::Update => JobKindData::Update {
                llms_txt: "# Test\n\n> Test content\n\n- [Link](/)".to_string(),
            },
        },
        status,
    )
    .await
}

/// Create a test job with specific JobKindData
///
/// This allows creating Update jobs with custom existing llms.txt content.
pub async fn create_test_job_with_kind_data(
    pool: &DbPool,
    url: &str,
    kind_data: JobKindData,
    status: JobStatus,
) -> JobState {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    let job_id = Uuid::new_v4();
    let new_job = JobState::from_kind_data(job_id, url.to_string(), status, kind_data);

    diesel::insert_into(schema::job_state::table)
        .values(&new_job)
        .execute(&mut conn)
        .await
        .expect("Failed to insert test job");

    new_job
}

/// Create a completed test job with llms.txt result
///
/// This creates both a JobState (with Success status) and a corresponding LlmsTxt record.
pub async fn create_completed_test_job(
    pool: &DbPool,
    url: &str,
    llms_txt_content: &str,
    html: &str,
) -> (JobState, LlmsTxt) {
    let job = create_test_job(pool, url, JobKind::New, JobStatus::Success).await;

    let llms_txt_record = LlmsTxt::from_result(
        job.job_id,
        url.to_string(),
        LlmsTxtResult::Ok {
            llms_txt: llms_txt_content.to_string(),
        },
        html.to_string(),
    );

    let mut conn = pool.get().await.expect("Failed to get database connection");

    diesel::insert_into(schema::llms_txt::table)
        .values(&llms_txt_record)
        .execute(&mut conn)
        .await
        .expect("Failed to insert llms_txt record");

    (job, llms_txt_record)
}

/// Create a failed test job with error result
///
/// Creates a JobState with Failure status and a corresponding LlmsTxt record with error.
/// If HTML is provided, it's stored (generation failure); otherwise it's not (download failure).
pub async fn create_failed_test_job(
    pool: &DbPool,
    url: &str,
    error_message: &str,
    html: Option<&str>,
) -> (JobState, Option<LlmsTxt>) {
    let job = create_test_job(pool, url, JobKind::New, JobStatus::Failure).await;

    let llms_txt_record = html.map(|html_content| {
        LlmsTxt::from_result(
            job.job_id,
            url.to_string(),
            LlmsTxtResult::Error {
                failure_reason: error_message.to_string(),
            },
            html_content.to_string(),
        )
    });

    if let Some(ref record) = llms_txt_record {
        let mut conn = pool.get().await.expect("Failed to get database connection");

        diesel::insert_into(schema::llms_txt::table)
            .values(record)
            .execute(&mut conn)
            .await
            .expect("Failed to insert llms_txt error record");
    }

    (job, llms_txt_record)
}

/// Seed the test database with sample data
///
/// Creates several test jobs in various states for integration testing:
/// - Queued jobs (both New and Update)
/// - Running job
/// - Completed successful jobs
/// - Failed job
pub async fn seed_test_data(pool: &DbPool) {
    // Queued jobs
    create_test_job(pool, "https://example.com", JobKind::New, JobStatus::Queued).await;
    create_test_job(pool, "https://test.com", JobKind::New, JobStatus::Queued).await;

    // Running job
    create_test_job(pool, "https://inprogress.com", JobKind::New, JobStatus::Running).await;

    // Completed jobs
    create_completed_test_job(
        pool,
        "https://completed.com",
        "# Completed Site\n\n> A completed test site\n\n- [Home](/)",
        "<html><body><h1>Completed</h1></body></html>",
    )
    .await;

    create_completed_test_job(
        pool,
        "https://another-completed.com",
        "# Another Site\n\n> Another test\n\n- [Home](/)\n- [About](/about)",
        "<html><body><h1>Another</h1></body></html>",
    )
    .await;

    // Failed job
    create_failed_test_job(
        pool,
        "https://failed.com",
        "Test failure reason",
        Some("<html><body>Failed HTML</body></html>"),
    )
    .await;

    // Update job (queued)
    create_test_job_with_kind_data(
        pool,
        "https://update-test.com",
        JobKindData::Update {
            llms_txt: "# Old Content\n\n> Old\n\n- [Link](/)".to_string(),
        },
        JobStatus::Queued,
    )
    .await;
}

/// Get a job by ID from the database
pub async fn get_job_by_id(pool: &DbPool, job_id: Uuid) -> Option<JobState> {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    schema::job_state::table
        .find(job_id)
        .first::<JobState>(&mut conn)
        .await
        .ok()
}

/// Get an llms_txt record by job ID
pub async fn get_llms_txt_by_job_id(pool: &DbPool, job_id: Uuid) -> Option<LlmsTxt> {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    schema::llms_txt::table
        .find(job_id)
        .first::<LlmsTxt>(&mut conn)
        .await
        .ok()
}

/// Count jobs with a specific status
pub async fn count_jobs_with_status(pool: &DbPool, status: JobStatus) -> i64 {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    schema::job_state::table
        .filter(schema::job_state::status.eq(status))
        .count()
        .get_result(&mut conn)
        .await
        .expect("Failed to count jobs")
}

/// Get all jobs with a specific status
pub async fn get_jobs_with_status(pool: &DbPool, status: JobStatus) -> Vec<JobState> {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    schema::job_state::table
        .filter(schema::job_state::status.eq(status))
        .load::<JobState>(&mut conn)
        .await
        .expect("Failed to load jobs")
}

/// Update a job's status
pub async fn update_job_status(pool: &DbPool, job_id: Uuid, new_status: JobStatus) {
    let mut conn = pool.get().await.expect("Failed to get database connection");

    diesel::update(schema::job_state::table.find(job_id))
        .set(schema::job_state::status.eq(new_status))
        .execute(&mut conn)
        .await
        .expect("Failed to update job status");
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::ResultStatus;
    use tokio::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

    #[tokio::test]
    async fn test_create_and_get_job() {
        let _db = TestDbGuard::acquire().await;
        let pool = test_db_pool().await;
        let _guard = TEST_MUTEX.lock().await;
        clean_test_db(&pool).await;

        let job = create_test_job(&pool, "https://test.com", JobKind::New, JobStatus::Queued).await;

        let retrieved = get_job_by_id(&pool, job.job_id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.url, "https://test.com");
        assert_eq!(retrieved.status, JobStatus::Queued);
        assert_eq!(retrieved.kind, JobKind::New);
    }

    #[tokio::test]
    async fn test_create_completed_job() {
        let _db = TestDbGuard::acquire().await;
        let pool = test_db_pool().await;
        let _guard = TEST_MUTEX.lock().await;
        clean_test_db(&pool).await;

        let (job, llms_txt) = create_completed_test_job(
            &pool,
            "https://test.com",
            "# Test\n\n> Test\n\n- [Link](/)",
            "<html></html>",
        )
        .await;

        assert_eq!(job.status, JobStatus::Success);

        let retrieved_llms_txt = get_llms_txt_by_job_id(&pool, job.job_id).await;
        assert!(retrieved_llms_txt.is_some());
        let retrieved_llms_txt = retrieved_llms_txt.unwrap();
        assert_eq!(retrieved_llms_txt.result_status, ResultStatus::Ok);
        assert_eq!(retrieved_llms_txt.html, "<html></html>");
        assert_eq!(retrieved_llms_txt, llms_txt);
    }

    #[tokio::test]
    async fn test_clean_test_db() {
        let _db = TestDbGuard::acquire().await;
        let pool = test_db_pool().await;
        let _guard = TEST_MUTEX.lock().await;

        // Create some test data
        create_test_job(&pool, "https://test1.com", JobKind::New, JobStatus::Queued).await;
        create_test_job(&pool, "https://test2.com", JobKind::New, JobStatus::Queued).await;

        let count_before = count_jobs_with_status(&pool, JobStatus::Queued).await;
        assert!(count_before >= 2);

        // Clean the database
        clean_test_db(&pool).await;

        let count_after = count_jobs_with_status(&pool, JobStatus::Queued).await;
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_update_job_status() {
        let _db = TestDbGuard::acquire().await;
        let pool = test_db_pool().await;
        let _guard = TEST_MUTEX.lock().await;
        clean_test_db(&pool).await;

        let job = create_test_job(&pool, "https://test.com", JobKind::New, JobStatus::Queued).await;
        assert_eq!(job.status, JobStatus::Queued);

        update_job_status(&pool, job.job_id, JobStatus::Running).await;

        let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
        assert_eq!(updated_job.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn test_get_jobs_with_status() {
        let _db = TestDbGuard::acquire().await;
        let pool = test_db_pool().await;
        let _guard = TEST_MUTEX.lock().await;
        clean_test_db(&pool).await;

        create_test_job(&pool, "https://test1.com", JobKind::New, JobStatus::Queued).await;
        create_test_job(&pool, "https://test2.com", JobKind::New, JobStatus::Queued).await;
        create_test_job(&pool, "https://test3.com", JobKind::New, JobStatus::Running).await;

        let queued_jobs = get_jobs_with_status(&pool, JobStatus::Queued).await;
        assert_eq!(queued_jobs.len(), 2);

        let running_jobs = get_jobs_with_status(&pool, JobStatus::Running).await;
        assert_eq!(running_jobs.len(), 1);
    }
}
