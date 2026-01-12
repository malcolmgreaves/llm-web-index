use std::{sync::Arc, time::Duration};

use core_ltx::{
    download, is_valid_url,
    llms::{ChatGpt, LlmProvider},
};
use data_model_ltx::{
    db,
    models::{JobKindData, JobState, JobStatus, LlmsTxt, LlmsTxtResult},
    schema,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cron_ltx::Error;

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
        let poll_interval_m = std::env::var("CRON_POLL_INTERVAL_M")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .expect("CRON_POLL_INTERVAL_M must be a valid number");

        tracing::info!(
            "Cron updater service started, polling every {} minutes",
            poll_interval_m
        );
        Duration::from_mins(poll_interval_m)
    };

    // Cron updater polling loop
    loop {
        // CLAUDE: get all records in the llms_txt table that are either success or failed
        // CLAUDE: for all failed ones, start tokio::spawn tasks to generate their llms.txt again (1st time)
        // CLAUDE: for all successful ones, start tokio::spawn tasks to
        //      (1) download the HTML from the url again
        //      (2) see if the new HTML matches exactly with the HTML stored in the llms_txt table
        //      (3) if there's any differences, hit the API server with an update llms.txt request
        // Once these tasks are spawned, wait for poll_interval duration
        tracing::info!("Waiting for {:?} until checking again.", poll_interval);
        tokio::time::sleep(poll_interval).await;
    }
}
