use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use core_ltx::db::DbPool;
use data_model_ltx::models::JobStatus;
use data_model_ltx::models::{
    JobDetailsResponse, JobIdPayload, JobState, JobStatusResponse, ResultStatus, StatusError,
};
use data_model_ltx::schema::{job_state, llms_txt};

/// Gets all currently running jobs for a given URL.
///
/// Returns all JobIds (UUID v4) of all in-progress jobs that match the `url`.
/// An in-progress job is one whose status is either Queued or Running.
///
/// An error is returned if there are no matching rows or if there's an internal DB error.
pub async fn in_progress_jobs(conn: &mut AsyncPgConnection, url: &str) -> Result<Vec<Uuid>, diesel::result::Error> {
    job_state::table
        .filter(job_state::url.eq(url))
        // only select currently running jobs
        .filter(job_state::status.eq_any(&[JobStatus::Queued, JobStatus::Queued, JobStatus::Running]))
        .select(job_state::job_id)
        .load::<Uuid>(conn)
        .await
}

// GET /api/status - Get the status of a job
pub async fn get_status(
    State(pool): State<DbPool>,
    Json(payload): Json<JobIdPayload>,
) -> Result<impl IntoResponse, StatusError> {
    let mut conn = pool.get().await?;

    let job = job_state::table
        .filter(job_state::job_id.eq(&payload.job_id))
        .select(JobState::as_select())
        .first::<JobState>(&mut conn)
        .await?;

    tracing::trace!("Success: retrieved status ({:?}) for job ({})", job.status, job.job_id);
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
    let mut conn = pool.get().await?;

    let job = job_state::table
        .filter(job_state::job_id.eq(&payload.job_id))
        .select(JobState::as_select())
        .first::<JobState>(&mut conn)
        .await?;

    // If the job failed, fetch the error message from llms_txt table
    let error_message = if job.status == JobStatus::Failure {
        llms_txt::table
            .filter(llms_txt::job_id.eq(&payload.job_id))
            .filter(llms_txt::result_status.eq(ResultStatus::Error))
            .select(llms_txt::result_data)
            .first::<String>(&mut conn)
            .await
            .ok()
    } else {
        None
    };

    let response = JobDetailsResponse {
        job_id: job.job_id,
        url: job.url,
        status: job.status,
        kind: job.kind,
        llms_txt: job.llms_txt,
        error_message,
    };

    tracing::trace!("Success: retrieved details for job ({})", job.job_id);
    Ok((StatusCode::OK, Json(response)))
}

// GET /api/jobs/in_progress - List all in-progress jobs
pub async fn get_in_progress_jobs(State(pool): State<DbPool>) -> Result<impl IntoResponse, StatusError> {
    let span = tracing::debug_span!("/api/jobs/in_progress");
    let _span = span.enter();

    let mut conn = pool.get().await?;

    let jobs = job_state::table
        .filter(job_state::status.eq_any(&[JobStatus::Queued, JobStatus::Running]))
        .select(JobState::as_select())
        .load::<JobState>(&mut conn)
        .await?;

    tracing::trace!("Success: retrieved all {} in-progress jobs", jobs.len());
    Ok((StatusCode::OK, Json(jobs)))
}
