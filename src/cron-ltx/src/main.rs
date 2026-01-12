use std::time::Duration;

use data_model_ltx::db;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "cron_ltx=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    let pool = db::establish_connection_pool(&database_url)
        .unwrap_or_else(|_| panic!("Couldn't connect to database: {}", database_url));

    let poll_interval = {
        let poll_interval_s = std::env::var("CRON_POLL_INTERVAL_S")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .expect("CRON_POLL_INTERVAL_S must be a valid number");

        tracing::info!(
            "Cron updater service started, polling every {} seconds",
            poll_interval_s
        );
        Duration::from_secs(poll_interval_s)
    };

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let api_base_url = {
        let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid number");
        format!("http://{}:{}", host, port)
    };

    tracing::info!("API server URL: {}", api_base_url);

    // Cron updater polling loop
    loop {
        tracing::info!("Starting cron poll cycle");

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
