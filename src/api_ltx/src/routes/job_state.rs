use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use diesel::prelude::*;
use serde_json::json;
use std::net::SocketAddr;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::schema::{job_state, llms_txt};
use db::{DbPool, establish_connection_pool};
use models::{
    GetLlmTxtError, JobIdPayload, JobIdResponse, JobKindData, JobState, JobStatus as JobStatusEnum,
    JobStatusResponse, LlmTxtResponse, LlmsTxt, LlmsTxtListItem, LlmsTxtListResponse,
    PostLlmTxtError, PutLlmTxtError, ResultStatus, StatusError, UpdateLlmTxtError, UrlPayload,
};

// GET /api/status - Get the status of a job
async fn get_status(
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
