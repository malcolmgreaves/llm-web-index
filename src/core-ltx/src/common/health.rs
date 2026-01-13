use axum::{Router, http::StatusCode};

pub async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "healthy")
}

pub fn health_router() -> Router {
    Router::new().route("/health", axum::routing::get(health_check))
}
