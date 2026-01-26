use std::sync::Arc;
use std::time::Duration;
use std::{env, path::PathBuf};

use core_ltx::common::env_check::check_non_empty_env_vars;
use core_ltx::{
    TimeUnit, get_api_base_url, get_auth_config, get_db_pool, get_poll_interval, is_auth_enabled, setup_logging,
};
use cron_ltx::AuthenticatedClient;
use data_model_ltx::db::DbPool;

#[tokio::main]
async fn main() {
    // Install the default crypto provider for rustls (required for HTTPS)
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    // Fail-fast check: verify required auth env vars are present if auth is enabled
    if is_auth_enabled() {
        check_non_empty_env_vars(&["AUTH_PASSWORD_HASH", "SESSION_SECRET", "TLS_KEY_PATH", "TLS_CERT_PATH"]);

        // we know these are non-empty -- ok to unwrap
        for var_name in &["TLS_KEY_PATH", "TLS_CERT_PATH"] {
            let val = env::var(var_name).unwrap();
            let path = PathBuf::from(val);
            if !path.is_file() {
                eprintln!(
                    "FATAL: {} points to file {}, but the file does not exist!",
                    var_name,
                    path.display()
                );
                std::process::exit(1);
            }
        }
    }

    setup_logging("cron_ltx=debug");

    let pool = get_db_pool().await;

    let poll_interval = get_poll_interval(TimeUnit::Seconds, "CRON_POLL_INTERVAL_S", 300);
    tracing::info!("Using a {:?} interval for updating.", poll_interval);

    // Load auth configuration
    let auth_config = get_auth_config();
    let password = auth_config.as_ref().and_then(|cfg| cfg.password.clone());

    if password.is_some() {
        tracing::info!("Authentication enabled for cron service");
    } else {
        tracing::info!("Authentication not enabled for cron service");
    }

    // Check if we should accept invalid TLS certificates (for development with self-signed certs)
    let accept_invalid_certs = env::var("ACCEPT_INVALID_CERTS")
        .map(|v| {
            let v = v.to_lowercase();
            v == "true" || v == "1"
        })
        .unwrap_or(false);

    let reqwest_client = if accept_invalid_certs {
        tracing::warn!("Accepting invalid TLS certificates (development mode)");
        reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(30))
            .build()
    } else {
        reqwest::Client::builder().timeout(Duration::from_secs(30)).build()
    }
    .expect("Failed to build HTTP client");

    let api_base_url = format!("https://{}", get_api_base_url());
    tracing::info!("API server URL: {}", api_base_url);

    let http_client = Arc::new(AuthenticatedClient::new(reqwest_client, api_base_url.clone(), password));

    // Authenticate immediately if password is configured
    if http_client.authenticate().await.is_ok() {
        tracing::info!("Initial authentication successful");
    } else {
        tracing::error!("Auth enabled but initial authentication failed!");
    }

    updater_loop(pool, http_client, api_base_url, poll_interval).await;
}

async fn updater_loop(
    pool: DbPool,
    http_client: Arc<AuthenticatedClient>,
    api_base_url: String,
    poll_interval: Duration,
) {
    tracing::info!("Starting llms.txt update loop.");
    loop {
        match cron_ltx::poll_and_process(&pool, &http_client, &api_base_url).await {
            Ok(num_spawned) => {
                tracing::info!("Spawned {} tasks for processing", num_spawned);
            }
            Err(e) => {
                tracing::error!("Error during poll cycle: {}", e);
            }
        }

        tracing::info!("Sleeping for {:?} until next poll", poll_interval);
        tokio::time::sleep(poll_interval).await;
    }
}
