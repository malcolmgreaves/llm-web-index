use std::sync::Arc;
use std::time::Duration;

use core_ltx::{TimeUnit, get_api_base_url, get_auth_config, get_db_pool, get_poll_interval, setup_logging};
use cron_ltx::AuthenticatedClient;
use data_model_ltx::db::DbPool;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    setup_logging("cron_ltx=debug");

    let pool = get_db_pool().await;

    let poll_interval = get_poll_interval(TimeUnit::Seconds, "CRON_POLL_INTERVAL_S", 300);
    tracing::info!("Using a {:?} interval for updating.", poll_interval);

    // Load auth configuration
    let auth_config = get_auth_config();
    let password = auth_config.as_ref().map(|cfg| cfg.password_hash.clone());

    if password.is_some() {
        tracing::info!("Authentication enabled for cron service");
    } else {
        tracing::info!("Authentication not enabled for cron service");
    }

    let reqwest_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let api_base_url = format!("http://{}", get_api_base_url());
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
