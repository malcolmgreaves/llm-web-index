use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::db::DbPool;
use crate::models::{JobIdPayload, JobKind, JobState, JobStatus, JobStatusResponse, StatusError};

/// Gets all currently running jobs for a given URL.
///
/// Returns all JobIds (UUID v4) of all in-progress jobs that match the `url`.
/// An in-progress job is one whose status is either Queued, Started, or Running.
///
/// An error is returned if there are no matching rows or if there's an internal DB error.
pub async fn in_progress_jobs(
    executor: impl sqlx::PgExecutor<'_>,
    url: &str,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT job_id
        FROM job_state
        WHERE url = $1
          AND status = ANY($2)
        "#,
        url,
        &[JobStatus::Queued, JobStatus::Started, JobStatus::Running] as &[JobStatus]
    )
    .fetch_all(executor)
    .await
}

// GET /api/status - Get the status of a job
pub async fn get_status(
    State(pool): State<DbPool>,
    Json(payload): Json<JobIdPayload>,
) -> Result<impl IntoResponse, StatusError> {
    let job = sqlx::query_as!(
        JobState,
        r#"
        SELECT job_id, url, status AS "status: JobStatus", kind AS "kind: JobKind", llms_txt
        FROM job_state
        WHERE job_id = $1
        "#,
        payload.job_id
    )
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(JobStatusResponse {
            status: job.status,
            kind: job.kind,
        }),
    ))
}

// GET /api/job - Get full job details by job_id
pub async fn get_job(
    State(pool): State<DbPool>,
    Query(payload): Query<JobIdPayload>,
) -> Result<impl IntoResponse, StatusError> {
    let job = sqlx::query_as!(
        JobState,
        r#"
        SELECT job_id, url, status AS "status: JobStatus", kind AS "kind: JobKind", llms_txt
        FROM job_state
        WHERE job_id = $1
        "#,
        payload.job_id
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::OK, Json(job)))
}

// GET /api/jobs/in_progress - List all in-progress jobs
pub async fn get_in_progress_jobs(
    State(pool): State<DbPool>,
) -> Result<impl IntoResponse, StatusError> {
    let jobs = sqlx::query_as!(
        JobState,
        r#"
        SELECT job_id, url, status AS "status: JobStatus", kind AS "kind: JobKind", llms_txt
        FROM job_state
        WHERE status = ANY($1)
        "#,
        &[JobStatus::Queued, JobStatus::Started, JobStatus::Running] as &[JobStatus]
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(jobs)))
}
