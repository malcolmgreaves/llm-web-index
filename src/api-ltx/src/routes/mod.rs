use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use core_ltx::{AuthConfig, health_check};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use data_model_ltx::db::DbPool;

use crate::auth;

pub mod job_state;
pub mod llms_txt;
pub mod logging_middleware;

//
// Router
//

pub fn router(auth_config: Option<AuthConfig>) -> Router<DbPool> {
    let auth_config_arc = Arc::new(auth_config);

    // Public auth routes (no authentication required)
    let auth_routes = Router::new()
        .route("/api/auth/login", post(auth::post_login))
        .route("/api/auth/logout", post(auth::post_logout))
        .route("/api/auth/check", get(auth::get_check))
        .with_state(auth_config_arc.clone());

    // Protected API routes (authentication required when enabled)
    let protected_routes = Router::new()
        .route("/api/llm_txt", get(llms_txt::get_llm_txt))
        .route("/api/llm_txt", post(llms_txt::post_llm_txt))
        .route("/api/llm_txt", put(llms_txt::put_llm_txt))
        .route("/api/update", post(llms_txt::post_update))
        .route("/api/list", get(llms_txt::get_list))
        .route("/api/status", get(job_state::get_status))
        .route("/api/job", get(job_state::get_job))
        .route("/api/jobs/in_progress", get(job_state::get_in_progress_jobs))
        .route_layer(middleware::from_fn_with_state(
            auth_config_arc.clone(),
            auth::require_auth,
        ));

    // Combine all routes
    Router::new()
        .route("/health", get(health_check))
        .merge(auth_routes)
        .merge(protected_routes)
        // Serve static assets from frontend pkg directory (no auth required)
        .nest_service("/pkg", ServeDir::new("src/front-ltx/www/pkg"))
        // Fallback to index.html for all other routes (enables client-side routing, no auth required)
        .fallback_service(ServeFile::new("src/front-ltx/www/index.html"))
        // Custom route access logging
        .layer(middleware::from_fn(logging_middleware::log_route_access))
        // Tracing middleware
        .layer(TraceLayer::new_for_http())
}
