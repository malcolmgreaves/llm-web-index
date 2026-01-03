use axum::{
    Router,
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use r2d2;
use serde_json::json;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::db::DbPool;
use crate::models::{
    GetLlmTxtError, PostLlmTxtError, PutLlmTxtError, StatusError, UpdateLlmTxtError,
};

pub mod job_state;
pub mod llms_txt;

//
// Router
//

pub fn router() -> Router<DbPool> {
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

//
// Error handling
//

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

macro_rules! from_error {
    ($lib_err:path, $err_type:tt) => {
        /// Converts a `$lib_err` into an `$err_type::Unknown`.
        impl From<$lib_err> for $err_type {
            fn from(e: $lib_err) -> Self {
                $err_type::Unknown(format!("{:?}", e))
            }
        }
    };
}

macro_rules! from_diesel_not_found_error {
    ($err_type:tt) => {
        /// Converts a `diesel::result::Error::NotFound` into an `$err_type::NotGenerated`
        /// otherwise it's a `$err_type::Unknown(diesel::result::Error)`.
        impl From<diesel::result::Error> for $err_type {
            fn from(e: diesel::result::Error) -> Self {
                match e {
                    diesel::result::Error::NotFound => $err_type::NotGenerated,
                    _ => $err_type::Unknown(format!("{:?}", e)),
                }
            }
        }
    };
}

// GetLlmTxtError

impl IntoResponse for GetLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            GetLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            GetLlmTxtError::Unknown(_) | GetLlmTxtError::GenerationFailure(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (status, Json(self)).into_response()
    }
}

from_error!(r2d2::Error, GetLlmTxtError);
from_diesel_not_found_error!(GetLlmTxtError);

// PostLlmTxtError

impl IntoResponse for PostLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            PostLlmTxtError::AlreadyGenerated | PostLlmTxtError::JobsInProgress(_) => {
                StatusCode::CONFLICT
            }
            PostLlmTxtError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(r2d2::Error, PostLlmTxtError);
from_error!(diesel::result::Error, PostLlmTxtError);

// PutLlmTxtError

impl IntoResponse for PutLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(self)).into_response()
    }
}

from_error!(r2d2::Error, PutLlmTxtError);
from_error!(diesel::result::Error, PutLlmTxtError);

// UpdateLlmTxtError

impl IntoResponse for UpdateLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            UpdateLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            UpdateLlmTxtError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(r2d2::Error, UpdateLlmTxtError);
from_diesel_not_found_error!(UpdateLlmTxtError);

// StatusError

impl IntoResponse for StatusError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            StatusError::InvalidId => StatusCode::BAD_REQUEST,
            StatusError::UnknownId => StatusCode::NOT_FOUND,
            StatusError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(r2d2::Error, StatusError);
from_error!(diesel::result::Error, StatusError);
