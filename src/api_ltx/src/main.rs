mod db;
mod models;
mod schema;

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

use db::{DbPool, establish_connection_pool};
use models::{
    GetLlmTxtError, JobIdPayload, JobIdResponse, JobKindData, JobState, JobStatus as JobStatusEnum,
    JobStatusResponse, LlmTxtResponse, LlmsTxt, LlmsTxtListItem, LlmsTxtListResponse,
    PostLlmTxtError, PutLlmTxtError, ResultStatus, StatusError, UpdateLlmTxtError, UrlPayload,
};
use schema::{job_state, llms_txt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_ltx=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    // Establish database connection pool
    let pool = establish_connection_pool(&database_url);

    // Build the router
    let app = Router::new()
        // API routes for llms.txt management
        .route("/api/llm_txt", get(get_llm_txt))
        .route("/api/llm_txt", post(post_llm_txt))
        .route("/api/llm_txt", put(put_llm_txt))
        .route("/api/status", get(get_status))
        .route("/api/update", post(post_update))
        .route("/api/list", get(get_list))
        // Serve static assets from frontend pkg directory
        .nest_service("/pkg", ServeDir::new("src/front_ltx/www/pkg"))
        // Fallback to index.html for all other routes (enables client-side routing)
        .fallback_service(ServeFile::new("src/front_ltx/www/index.html"))
        // Middleware
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    // Define the address to listen on
    // Use HOST and PORT environment variables, defaulting to 127.0.0.1:3000
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid HOST or PORT");
    tracing::info!("Listening on {}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// GET /api/llm_txt - Retrieve llms.txt content for a URL
async fn get_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, GetLlmTxtError> {
    let mut conn = pool.get().map_err(|_| GetLlmTxtError::Unknown)?;

    // Get the most recent record for this URL (ordered by created_at DESC)
    let result = llms_txt::table
        .filter(llms_txt::url.eq(&payload.url))
        .order(llms_txt::created_at.desc())
        .select(LlmsTxt::as_select())
        .first::<LlmsTxt>(&mut conn)
        .optional()
        .map_err(|_| GetLlmTxtError::Unknown)?;

    match result {
        Some(llms_txt_record) => match llms_txt_record.result_status {
            models::ResultStatus::Ok => Ok((
                StatusCode::OK,
                Json(LlmTxtResponse {
                    content: llms_txt_record.result_data,
                }),
            )),
            models::ResultStatus::Error => Err(GetLlmTxtError::NotGenerated),
        },
        None => Err(GetLlmTxtError::NotGenerated),
    }
}

// POST /api/llm_txt - Create a new job to generate llms.txt
async fn post_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PostLlmTxtError> {
    let mut conn = pool.get().map_err(|_| PostLlmTxtError::Unknown)?;

    // Check if llms.txt already exists for this URL
    let existing = llms_txt::table
        .filter(llms_txt::url.eq(&payload.url))
        .select(LlmsTxt::as_select())
        .first::<LlmsTxt>(&mut conn)
        .optional()
        .map_err(|_| PostLlmTxtError::Unknown)?;

    if existing.is_some() {
        return Err(PostLlmTxtError::AlreadyGenerated);
    }

    // Create a new job
    let job_id = uuid::Uuid::new_v4();
    let new_job =
        JobState::from_kind_data(job_id, payload.url, JobStatusEnum::Queued, JobKindData::New);

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(&mut conn)
        .map_err(|_| PostLlmTxtError::Unknown)?;

    Ok((StatusCode::CREATED, Json(JobIdResponse { job_id })))
}

// PUT /api/llm_txt - Create a new job (unconditionally)
async fn put_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PutLlmTxtError> {
    let mut conn = pool.get().map_err(|_| PutLlmTxtError::Unknown)?;

    // Check if there's an existing entry for the url in the llms_txt table with Ok result
    let existing = llms_txt::table
        .filter(llms_txt::url.eq(&payload.url))
        .order(llms_txt::created_at.desc())
        .select(LlmsTxt::as_select())
        .first::<LlmsTxt>(&mut conn)
        .optional()
        .map_err(|_| PutLlmTxtError::Unknown)?;

    // Determine job kind and status code based on existing data
    let (kind_data, status_code) = match existing {
        Some(llms_txt_record) if llms_txt_record.result_status == ResultStatus::Ok => {
            // If there's an Ok result, create an Update job (like POST /api/update)
            (
                JobKindData::Update {
                    llms_txt: llms_txt_record.result_data,
                },
                StatusCode::OK,
            )
        }
        _ => {
            // Otherwise, create a New job (like POST /api/llm_txt)
            // This covers: no existing entry OR existing entry with Error status
            (JobKindData::New, StatusCode::CREATED)
        }
    };

    // Create the job
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(job_id, payload.url, JobStatusEnum::Queued, kind_data);

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(&mut conn)
        .map_err(|_| PutLlmTxtError::Unknown)?;

    Ok((status_code, Json(JobIdResponse { job_id })))
}

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

// POST /api/update - Create an update job for existing llms.txt
async fn post_update(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, UpdateLlmTxtError> {
    let mut conn = pool.get().map_err(|_| UpdateLlmTxtError::Unknown)?;

    // Check if llms.txt exists for this URL and get the most recent one
    let existing = llms_txt::table
        .filter(llms_txt::url.eq(&payload.url))
        .order(llms_txt::created_at.desc())
        .select(LlmsTxt::as_select())
        .first::<LlmsTxt>(&mut conn)
        .optional()
        .map_err(|_| UpdateLlmTxtError::Unknown)?;

    match existing {
        Some(llms_txt_record) => {
            // Ensure it's an Ok result (not Error)
            if llms_txt_record.result_status != ResultStatus::Ok {
                return Err(UpdateLlmTxtError::NotGenerated);
            }

            // Create an update job using the existing llms.txt result_data
            let job_id = uuid::Uuid::new_v4();
            let new_job = JobState::from_kind_data(
                job_id,
                payload.url,
                JobStatusEnum::Queued,
                JobKindData::Update {
                    llms_txt: llms_txt_record.result_data,
                },
            );

            diesel::insert_into(job_state::table)
                .values(&new_job)
                .execute(&mut conn)
                .map_err(|_| UpdateLlmTxtError::Unknown)?;

            Ok((StatusCode::CREATED, Json(JobIdResponse { job_id })))
        }
        None => Err(UpdateLlmTxtError::NotGenerated),
    }
}

// GET /api/list - List all successfully fetched llms.txt files
async fn get_list(State(pool): State<DbPool>) -> Result<impl IntoResponse, AppError> {
    use std::collections::HashMap;

    let mut conn = pool.get()?;

    // Load all Ok records ordered by url and created_at DESC
    let all_records = llms_txt::table
        .filter(llms_txt::result_status.eq(ResultStatus::Ok))
        .order((llms_txt::url.asc(), llms_txt::created_at.desc()))
        .select(LlmsTxt::as_select())
        .load::<LlmsTxt>(&mut conn)?;

    // Deduplicate by URL, keeping only the most recent
    let mut url_map: HashMap<String, String> = HashMap::new();
    for record in all_records {
        url_map.entry(record.url).or_insert(record.result_data);
    }

    // Convert to list response
    let items: Vec<LlmsTxtListItem> = url_map
        .into_iter()
        .map(|(url, llm_txt)| LlmsTxtListItem { url, llm_txt })
        .collect();

    Ok((StatusCode::OK, Json(LlmsTxtListResponse { items })))
}

// Error handling
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// Custom error type IntoResponse implementations

impl IntoResponse for GetLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            GetLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            GetLlmTxtError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

impl IntoResponse for PostLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            PostLlmTxtError::AlreadyGenerated => StatusCode::CONFLICT,
            PostLlmTxtError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

impl IntoResponse for PutLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(self)).into_response()
    }
}

impl IntoResponse for StatusError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            StatusError::InvalidId => StatusCode::BAD_REQUEST,
            StatusError::UnknownId => StatusCode::NOT_FOUND,
            StatusError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

impl IntoResponse for UpdateLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            UpdateLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            UpdateLlmTxtError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}
