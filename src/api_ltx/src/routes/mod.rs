pub mod job_state;
pub mod llms_txt;

pub fn router() -> Router {
    Router::new()
        // API routes for llms.txt management
        .route("/api/llm_txt", get(llms_txt::get_llm_txt))
        .route("/api/llm_txt", post(llms_txt::post_llm_txt))
        .route("/api/llm_txt", put(llms_txt::put_llm_txt))
        .route("/api/update", post(llms_txt::post_update))
        .route("/api/list", get(llms_txt::get_list))
        .route("/api/status", get(job_state::get_status))
        // Serve static assets from frontend pkg directory
        .nest_service("/pkg", ServeDir::new("src/front_ltx/www/pkg"))
        // Fallback to index.html for all other routes (enables client-side routing)
        .fallback_service(ServeFile::new("src/front_ltx/www/index.html"))
        // Middleware
        .layer(TraceLayer::new_for_http())
}

// Error handling
pub struct AppError(anyhow::Error);

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
