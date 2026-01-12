use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use api_ltx::routes;
use data_model_ltx::db::establish_connection_pool;

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
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    // Establish database connection pool
    let pool = establish_connection_pool(&database_url)
        .unwrap_or_else(|_| panic!("Couldn't connect to database: {}", database_url));

    // Build the router
    let app = routes::router().with_state(pool);

    let addr = core_ltx::get_api_base_url().expect("Invalid HOST or PORT");
    tracing::info!("Listening on {}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
