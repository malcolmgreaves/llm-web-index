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
        .route("/api/llm_txt", get(routes::llms_txt::get_llm_txt))
        .route("/api/llm_txt", post(routes::llms_txt::post_llm_txt))
        .route("/api/llm_txt", put(routes::llms_txt::put_llm_txt))
        .route("/api/update", post(routes::llms_txt::post_update))
        .route("/api/status", get(routes::job_state::get_status))
        .route("/api/list", get(routes::llms_txt::get_list))
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
