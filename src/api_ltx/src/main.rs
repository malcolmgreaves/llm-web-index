mod db;
mod models;
mod schema;

use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use diesel::prelude::*;
use serde_json::json;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use db::{DbPool, establish_connection_pool};
use models::{Name, NewName};
use schema::names;

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
        .route("/hello", get(hello))
        .route("/add", post(add_name))
        .route("/fetch", get(fetch_names))
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

// GET /hello - Returns "Hello world!"
async fn hello() -> &'static str {
    "Hello world!"
}

// POST /add - Adds a name to the database
async fn add_name(
    State(pool): State<DbPool>,
    Json(payload): Json<NewName>,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = pool.get()?;

    let new_name = diesel::insert_into(names::table)
        .values(&payload)
        .get_result::<Name>(&mut conn)?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": new_name.id,
            "name": new_name.name,
            "message": "Name added successfully"
        })),
    ))
}

// GET /fetch - Retrieves all names from the database
async fn fetch_names(State(pool): State<DbPool>) -> Result<impl IntoResponse, AppError> {
    let mut conn = pool.get()?;

    let results = names::table.select(Name::as_select()).load(&mut conn)?;

    Ok(Json(json!({
        "names": results,
        "count": results.len()
    })))
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
