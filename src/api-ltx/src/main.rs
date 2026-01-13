use std::net::SocketAddr;

use core_ltx::{get_api_base_url, get_auth_config, get_db_pool, setup_logging};
use tracing::info;

use api_ltx::routes;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    setup_logging("api_ltx=debug,tower_http=debug");

    // Load authentication configuration
    let auth_config = get_auth_config();
    if auth_config.is_some() {
        info!("Authentication: ENABLED");
    } else {
        info!("Authentication: DISABLED");
    }

    let pool = get_db_pool().await;
    let app = routes::router(auth_config).with_state(pool);

    let addr = get_api_base_url()
        .parse::<SocketAddr>()
        .expect("Expected a socket address!");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to address: {}", addr));

    axum::serve(listener, app).await.unwrap();
}
