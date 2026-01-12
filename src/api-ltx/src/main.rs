use core_ltx::{get_api_base_url, get_db_pool, setup_logging};

use api_ltx::routes;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    setup_logging("api_ltx=debug,tower_http=debug");

    let pool = get_db_pool();
    let app = routes::router().with_state(pool);

    let addr = get_api_base_url().expect("Invalid HOST or PORT");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(format!("Failed to bind to address: {}", addr).as_str());
    axum::serve(listener, app).await.unwrap();
}
