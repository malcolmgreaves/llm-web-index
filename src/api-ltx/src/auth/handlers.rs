use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use core_ltx::AuthConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{Duration, sleep};
use tracing::{debug, warn};

use super::password::verify_password;
use super::session::{
    create_logout_cookie, create_session_cookie, generate_session_token, parse_session_cookie, validate_session_token,
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    success: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthCheckResponse {
    auth_enabled: bool,
    authenticated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Password error: {0}")]
    PasswordError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AuthError::SessionError(_) | AuthError::PasswordError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Authentication error")
            }
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

/// POST /api/auth/login
/// Authenticates user with password, enforces minimum 1-second response time
pub async fn post_login(
    State(auth_config): State<Arc<Option<AuthConfig>>>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let start = Instant::now();

    // Get auth config (should always be Some when this handler is reachable)
    let config = auth_config
        .as_ref()
        .as_ref()
        .ok_or_else(|| AuthError::SessionError("Auth not configured".to_string()))?;

    // Verify password using bcrypt
    let is_valid = verify_password(&request.password, &config.password_hash)
        .map_err(|e| AuthError::PasswordError(e.to_string()))?;

    // Ensure minimum 1 second elapsed (timing attack protection)
    let elapsed = start.elapsed();
    if elapsed < Duration::from_secs(1) {
        sleep(Duration::from_secs(1) - elapsed).await;
    }

    if !is_valid {
        warn!("Failed login attempt");
        return Err(AuthError::InvalidCredentials);
    }

    // Generate session token
    let token = generate_session_token(&config.session_secret).map_err(|e| AuthError::SessionError(e.to_string()))?;

    // Create session cookie
    let cookie = create_session_cookie(&token, config.session_duration_seconds);

    debug!("Successful login");

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
        Json(LoginResponse { success: true }),
    ))
}

/// POST /api/auth/logout
/// Clears the session cookie
pub async fn post_logout() -> impl IntoResponse {
    let cookie = create_logout_cookie();

    debug!("User logged out");

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
        Json(serde_json::json!({"success": true})),
    )
}

/// GET /api/auth/check
/// Returns authentication status
pub async fn get_check(
    State(auth_config): State<Arc<Option<AuthConfig>>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let auth_enabled = auth_config.is_some();

    let authenticated = if let Some(config) = auth_config.as_ref() {
        // Check if valid session cookie exists
        if let Some(cookie_header) = headers.get(header::COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                if let Some(token) = parse_session_cookie(cookie_str) {
                    validate_session_token(&token, &config.session_secret, config.session_duration_seconds)
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        // Auth not enabled, so user is implicitly authenticated
        true
    };

    Json(AuthCheckResponse {
        auth_enabled,
        authenticated,
    })
}
