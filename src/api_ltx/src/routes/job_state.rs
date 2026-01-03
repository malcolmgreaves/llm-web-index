use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;

use crate::db::DbPool;
use crate::models::{JobIdPayload, JobState, JobStatusResponse, StatusError};
use crate::schema::job_state;

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
