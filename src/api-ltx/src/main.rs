use std::net::SocketAddr;

use core_ltx::{get_api_base_url, get_auth_config, get_db_pool, get_tls_config, setup_logging};
use tracing::info;

use api_ltx::routes;

#[tokio::main]
async fn main() {
    // Install the default crypto provider for rustls (required for TLS)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

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

    // Load TLS configuration (REQUIRED)
    let tls_config = get_tls_config().await;
    info!("TLS: ENABLED");

    let pool = get_db_pool().await;
    let app = routes::router(auth_config).with_state(pool);

    let addr = get_api_base_url()
        .parse::<SocketAddr>()
        .expect("Expected a socket address!");

    info!("Starting HTTPS server on https://{}", addr);

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
