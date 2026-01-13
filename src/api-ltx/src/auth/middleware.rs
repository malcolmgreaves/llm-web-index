use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use core_ltx::AuthConfig;
use std::sync::Arc;
use tracing::debug;

use super::session::{parse_session_cookie, validate_session_token};

/// Middleware to require authentication when enabled
/// If auth is disabled, requests pass through immediately
/// If auth is enabled, validates session cookie
pub async fn require_auth(
    State(auth_config): State<Arc<Option<AuthConfig>>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // If auth is not enabled, pass through immediately
    let config = match auth_config.as_ref() {
        Some(cfg) => cfg,
        None => {
            debug!("Auth not enabled, passing request through");
            return Ok(next.run(request).await);
        }
    };

    let cookie_header = request.headers().get(header::COOKIE).and_then(|h| h.to_str().ok());

    let is_authenticated = if let Some(cookie_str) = cookie_header {
        if let Some(token) = parse_session_cookie(cookie_str) {
            validate_session_token(&token, &config.session_secret, config.session_duration_seconds).unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    if is_authenticated {
        debug!("Request authenticated");
        Ok(next.run(request).await)
    } else {
        debug!("Request not authenticated, returning 401");
        Err(unauthorized_response())
    }
}

fn unauthorized_response() -> Response {
    let body = Json(serde_json::json!({
        "error": "Authentication required"
    }));

    (StatusCode::UNAUTHORIZED, body).into_response()
}
