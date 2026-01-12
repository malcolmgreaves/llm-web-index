use axum::{
    Router,
    routing::{get, post, put},
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use data_model_ltx::db::DbPool;

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
        .route("/api/job", get(job_state::get_job))
        .route("/api/jobs/in_progress", get(job_state::get_in_progress_jobs))
        // Serve static assets from frontend pkg directory
        .nest_service("/pkg", ServeDir::new("src/front-ltx/www/pkg"))
        // Fallback to index.html for all other routes (enables client-side routing)
        .fallback_service(ServeFile::new("src/front-ltx/www/index.html"))
        // Middleware
        .layer(TraceLayer::new_for_http())
}
