use std::time::Duration;

use core_ltx::{TimeUnit, get_api_base_url, get_db_pool, get_poll_interval, setup_logging};
use data_model_ltx::db::DbPool;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    setup_logging("cron_ltx=debug");

    let pool = get_db_pool();

    let poll_interval = get_poll_interval(TimeUnit::Seconds, "CRON_POLL_INTERVAL_S", 300);
    tracing::info!("Using a {:?} interval for updating.", poll_interval);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let api_base_url = get_api_base_url().to_string();
    tracing::info!("API server URL: {}", api_base_url);

    updater_loop(pool, http_client, api_base_url, poll_interval).await;
}

async fn updater_loop(pool: DbPool, http_client: reqwest::Client, api_base_url: String, poll_interval: Duration) {
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
