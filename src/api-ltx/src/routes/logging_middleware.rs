use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// Middleware that logs each route access with its result
pub async fn log_route_access(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    // Call the actual route handler
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    // Log based on status code
    match status.as_u16() {
        200..=399 => {
            tracing::info!(
                method = %method,
                path = %path,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
            );
        }
        400..=499 => {
            tracing::warn!(
                method = %method,
                path = %path,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
            );
        }
        500..=599 => {
            tracing::error!(
                method = %method,
                path = %path,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
            );
        }
        _ => {
            tracing::info!(
                method = %method,
                path = %path,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
            );
        }
    }

    response
}
