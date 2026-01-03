use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{JobIdPayload, JobState, JobStatusResponse, StatusError};
use crate::schema::job_state;
use crate::{
    db::{Conn, DbPool},
    models::JobStatus,
};

/// Gets all currently running jobs for a given URL.
pub fn in_progress_jobs(conn: &mut Conn, url: &str) -> Result<Vec<Uuid>, diesel::result::Error> {
    job_state::table
        .filter(job_state::url.eq(url))
        // only select currently running jobs
        .filter(job_state::status.eq_any(&[
            JobStatus::Queued,
            JobStatus::Queued,
            JobStatus::Running,
        ]))
        .select(job_state::job_id)
        .load::<Uuid>(conn)
}

// GET /api/status - Get the status of a job
pub async fn get_status(
    State(pool): State<DbPool>,
    Json(payload): Json<JobIdPayload>,
) -> Result<impl IntoResponse, StatusError> {
    let mut conn = pool.get().map_err(|_| StatusError::Unknown)?;

    let job = job_state::table
        .filter(job_state::job_id.eq(&payload.job_id))
        .select(JobState::as_select())
        .first::<JobState>(&mut conn)
        .optional()
        .map_err(|_| StatusError::Unknown)?;

    match job {
        Some(job_state) => Ok((
            StatusCode::OK,
            Json(JobStatusResponse {
                status: job_state.status,
                kind: job_state.kind,
            }),
        )),
        None => Err(StatusError::UnknownId),
    }
}
