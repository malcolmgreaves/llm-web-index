use std::env;
use std::net::SocketAddr;

use core_ltx::{get_api_base_url, get_auth_config, get_db_pool, get_tls_config, is_auth_enabled, setup_logging};
use tracing::info;

use api_ltx::routes;

#[tokio::main]
async fn main() {
    // Install the default crypto provider for rustls (required for TLS)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Fail-fast check: verify required auth env vars are present if auth is enabled
    if is_auth_enabled() {
        let required_vars = ["AUTH_PASSWORD_HASH", "SESSION_SECRET"];
        for var_name in &required_vars {
            match env::var(var_name) {
                Ok(value) if !value.trim().is_empty() => {
                    // Variable is present and non-empty, continue
                }
                Ok(_) => {
                    eprintln!(
                        "FATAL: {} environment variable is set but empty. \
                         Authentication is enabled (ENABLE_AUTH=true) but required configuration is invalid.",
                        var_name
                    );
                    std::process::exit(1);
                }
                Err(_) => {
                    eprintln!(
                        "FATAL: {} environment variable is required when ENABLE_AUTH=true.",
                        var_name
                    );
                    if var_name == &"AUTH_PASSWORD_HASH" {
                        eprintln!("Generate a hash with: cargo run --bin generate-password-hash -- your_password");
                    } else if var_name == &"SESSION_SECRET" {
                        eprintln!("Generate a secret with: openssl rand -base64 32");
                    }
                    std::process::exit(1);
                }
            }
        }
    }

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
