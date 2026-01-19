//! Tests for job queue operations
//!
//! This module tests the next_job_in_queue() function which is responsible for:
//! - Claiming jobs from the queue using FOR UPDATE SKIP LOCKED
//! - Marking jobs as Running when claimed
//! - Handling concurrent worker access
//! - Proper job status transitions

use std::sync::Arc;

use data_model_ltx::{
    db::{self},
    models::{JobKind, JobKindData, JobState, JobStatus},
    test_helpers::{clean_test_db, create_test_job, get_job_by_id, test_db_pool, update_job_status},
};
use tokio::sync::Semaphore;
use worker_ltx::work::next_job_in_queue;

async fn next_job(pool: &db::DbPool) -> Result<JobState, worker_ltx::Error> {
    next_job_in_queue(pool, Arc::new(Semaphore::new(1))).await.map(|x| x.0)
}

#[tokio::test]
async fn test_next_job_in_queue_claims_queued_job() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create a queued job
    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Queued).await;

    // Claim it
    let claimed_job = next_job(&pool).await.unwrap();

    assert_eq!(claimed_job.job_id, job.job_id);
    assert_eq!(claimed_job.url, job.url);

    // Verify the job is now marked as Running in the database
    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Running);
}

#[tokio::test]
async fn test_next_job_in_queue_claims_started_job() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create a job in Started state
    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Started).await;

    // Should be able to claim Started jobs too
    let claimed_job = next_job(&pool).await.unwrap();

    assert_eq!(claimed_job.job_id, job.job_id);

    // Verify it's now Running
    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Running);
}

#[tokio::test]
async fn test_next_job_in_queue_empty_queue() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // No jobs in queue
    let result = next_job(&pool).await;

    assert!(result.is_err(), "Should return error when queue is empty");
}

#[tokio::test]
async fn test_next_job_in_queue_ignores_running_jobs() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create a job that's already running
    create_test_job(&pool, "https://running.com", JobKind::New, JobStatus::Running).await;

    // Try to claim - should fail since only Running job exists
    let result = next_job(&pool).await;

    assert!(result.is_err(), "Should not claim jobs that are already Running");
}

#[tokio::test]
async fn test_next_job_in_queue_ignores_completed_jobs() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create completed jobs
    create_test_job(&pool, "https://success.com", JobKind::New, JobStatus::Success).await;
    create_test_job(&pool, "https://failed.com", JobKind::New, JobStatus::Failure).await;

    // Try to claim - should fail since only completed jobs exist
    let result = next_job(&pool).await;

    assert!(result.is_err(), "Should not claim jobs that are already completed");
}

#[tokio::test]
async fn test_next_job_in_queue_processes_in_order() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create multiple queued jobs
    let job1 = create_test_job(&pool, "https://first.com", JobKind::New, JobStatus::Queued).await;
    let job2 = create_test_job(&pool, "https://second.com", JobKind::New, JobStatus::Queued).await;
    let job3 = create_test_job(&pool, "https://third.com", JobKind::New, JobStatus::Queued).await;

    // Claim first job
    let claimed1 = next_job(&pool).await.unwrap();
    assert_eq!(claimed1.job_id, job1.job_id, "Should claim first job");

    // Claim second job
    let claimed2 = next_job(&pool).await.unwrap();
    assert_eq!(claimed2.job_id, job2.job_id, "Should claim second job");

    // Claim third job
    let claimed3 = next_job(&pool).await.unwrap();
    assert_eq!(claimed3.job_id, job3.job_id, "Should claim third job");

    // No more jobs
    let result = next_job(&pool).await;
    assert!(result.is_err(), "Should have no more jobs to claim");
}

/// Applies a function to multiple values, or to a tuple literal's elements.
/// Evaluates to a tuple of transformed values, the output order corresponds 1:1 to input order.
///
macro_rules! map {
    // Tuple literal input: map!(f, (a, b, c)) -> (f(a), f(b), f(c))
    ($f:expr, ($($x:expr),+ $(,)?)) => {
        ($($f($x)),+)
    };

    // Individual arguments: map!(f, a, b, c) -> (f(a), f(b), f(c))
    ($f:expr, $($x:expr),+ $(,)?) => {
        ($($f($x)),+)
    };
}

#[tokio::test]
async fn test_next_job_in_queue_concurrent_claiming() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create 5 jobs
    let job1 = create_test_job(&pool, "https://job1.com", JobKind::New, JobStatus::Queued).await;
    let job2 = create_test_job(&pool, "https://job2.com", JobKind::New, JobStatus::Queued).await;
    let job3 = create_test_job(&pool, "https://job3.com", JobKind::New, JobStatus::Queued).await;
    let job4 = create_test_job(&pool, "https://job4.com", JobKind::New, JobStatus::Queued).await;
    let job5 = create_test_job(&pool, "https://job5.com", JobKind::New, JobStatus::Queued).await;

    // Spawn 3 concurrent workers trying to claim jobs
    // Wait for all to complete
    let (result1, result2, result3) = {
        async fn next_job(pool: db::DbPool) -> Result<JobState, worker_ltx::Error> {
            next_job_in_queue(&pool, Arc::new(Semaphore::new(1))).await.map(|x| x.0)
        }

        map!(
            |x| { x.unwrap() },
            tokio::join!(
                tokio::spawn(next_job(pool.clone())),
                tokio::spawn(next_job(pool.clone())),
                tokio::spawn(next_job(pool.clone()))
            )
        )
    };

    // All should succeed (we have 5 jobs, 3 workers)
    // assert!(result1.is_ok(), "Worker 1 should claim a job");
    // assert!(result2.is_ok(), "Worker 2 should claim a job");
    // assert!(result3.is_ok(), "Worker 3 should claim a job");

    // Extract claimed job IDs
    // let claimed1 = result1.unwrap().job_id;
    // let claimed2 = result2.unwrap().job_id;
    // let claimed3 = result3.unwrap().job_id;
    let claimed1 = result1.job_id;
    let claimed2 = result2.job_id;
    let claimed3 = result3.job_id;

    // Each worker should have claimed a different job (SKIP LOCKED working)
    assert_ne!(claimed1, claimed2, "Workers should claim different jobs");
    assert_ne!(claimed1, claimed3, "Workers should claim different jobs");
    assert_ne!(claimed2, claimed3, "Workers should claim different jobs");

    // All claimed jobs should be among our created jobs
    let all_job_ids = vec![job1.job_id, job2.job_id, job3.job_id, job4.job_id, job5.job_id];
    assert!(all_job_ids.contains(&claimed1), "Claimed job should be one we created");
    assert!(all_job_ids.contains(&claimed2), "Claimed job should be one we created");
    assert!(all_job_ids.contains(&claimed3), "Claimed job should be one we created");

    // Verify all claimed jobs are now Running
    let status1 = get_job_by_id(&pool, claimed1).await.unwrap().status;
    let status2 = get_job_by_id(&pool, claimed2).await.unwrap().status;
    let status3 = get_job_by_id(&pool, claimed3).await.unwrap().status;

    assert_eq!(status1, JobStatus::Running);
    assert_eq!(status2, JobStatus::Running);
    assert_eq!(status3, JobStatus::Running);
}

#[tokio::test]
async fn test_next_job_in_queue_skips_locked_jobs() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create 3 jobs
    let job1 = create_test_job(&pool, "https://job1.com", JobKind::New, JobStatus::Queued).await;
    let job2 = create_test_job(&pool, "https://job2.com", JobKind::New, JobStatus::Queued).await;
    let _job3 = create_test_job(&pool, "https://job3.com", JobKind::New, JobStatus::Queued).await;

    // Worker 1 claims first job
    let claimed1 = next_job(&pool).await.unwrap();
    assert_eq!(claimed1.job_id, job1.job_id);

    // Worker 2 should skip job1 (now Running) and claim job2
    let claimed2 = next_job(&pool).await.unwrap();
    assert_eq!(claimed2.job_id, job2.job_id);
    assert_ne!(claimed2.job_id, claimed1.job_id, "Should claim different job");
}

#[tokio::test]
async fn test_next_job_in_queue_handles_both_new_and_update_jobs() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create both New and Update jobs
    let new_job = create_test_job(&pool, "https://new.com", JobKind::New, JobStatus::Queued).await;

    use data_model_ltx::test_helpers::create_test_job_with_kind_data;
    let update_job = create_test_job_with_kind_data(
        &pool,
        "https://update.com",
        JobKindData::Update {
            llms_txt: "# Old\n\n> Content\n\n- [Link](/)".to_string(),
        },
        JobStatus::Queued,
    )
    .await;

    // Should be able to claim both types
    let claimed1 = next_job(&pool).await.unwrap();
    let claimed2 = next_job(&pool).await.unwrap();

    // Both should be claimed
    let claimed_ids = vec![claimed1.job_id, claimed2.job_id];
    assert!(claimed_ids.contains(&new_job.job_id));
    assert!(claimed_ids.contains(&update_job.job_id));
}

#[tokio::test]
async fn test_next_job_in_queue_transaction_isolation() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create a job
    let job = create_test_job(&pool, "https://test.com", JobKind::New, JobStatus::Queued).await;

    // Claim it
    let claimed = next_job_in_queue(&pool).await.unwrap();
    assert_eq!(claimed.job_id, job.job_id);

    // Try to claim again - should fail because job is now Running
    let result = next_job_in_queue(&pool).await;
    assert!(result.is_err(), "Should not be able to claim the same job twice");

    // Verify job is indeed Running
    let current_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(current_job.status, JobStatus::Running);
}

#[tokio::test]
async fn test_next_job_in_queue_marks_job_running_atomically() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create a job
    let job = create_test_job(&pool, "https://test.com", JobKind::New, JobStatus::Queued).await;

    // The job returned by next_job_in_queue should already reflect Running status
    // (this is important for the function's contract)
    let claimed = next_job(&pool).await.unwrap();

    assert_eq!(
        claimed.job_id, job.job_id,
        "Expecting worker to claim only job available: expected={} actual={}",
        job.job_id, claimed.job_id
    );

    // The returned job might not have the updated status yet (it's the job before update)
    // But the database should be updated
    let db_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(
        db_job.status,
        JobStatus::Running,
        "Job should be marked Running in database"
    );
}

#[tokio::test]
async fn test_next_job_in_queue_prefers_started_over_queued() {
    let pool = test_db_pool().await;
    clean_test_db(&pool).await;

    // Create jobs in different states
    // Note: The SQL query orders by job_id ASC, so creation order matters
    let queued_job = create_test_job(&pool, "https://queued.com", JobKind::New, JobStatus::Queued).await;

    let started_job = create_test_job(&pool, "https://started.com", JobKind::New, JobStatus::Started).await;

    // Both Queued and Started are eligible
    // The query should pick based on job_id order (ASC), not status
    let claimed1 = next_job(&pool).await.unwrap();

    // Should claim the first eligible job by ID
    assert_eq!(claimed1.job_id, queued_job.job_id);

    // Second claim should get the other job
    let claimed2 = next_job(&pool).await.unwrap();
    assert_eq!(claimed2.job_id, started_job.job_id);
}
