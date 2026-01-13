use std::{sync::Arc, time::Duration};

use core_ltx::{
    TimeUnit, get_db_pool, get_max_concurrency, get_poll_interval,
    llms::{ChatGpt, LlmProvider},
    setup_logging,
};
use data_model_ltx::db::DbPool;
use tokio::sync::Semaphore;
use worker_ltx::{Error, JobResult, handle_job, handle_result, next_job_in_queue};

#[tokio::main]
async fn main() {
    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    setup_logging("worker_ltx=debug");

    let provider: Arc<ChatGpt> = Arc::new(ChatGpt::default());

    let pool = get_db_pool().await;

    let poll_interval = get_poll_interval(TimeUnit::Milliseconds, "WORKER_POLL_INTERVAL_MS", 600);

    let max_concurrency = get_max_concurrency(None);
    tracing::info!("Worker configured with max concurrency: {}", max_concurrency);

    let semaphore = Arc::new(Semaphore::new(max_concurrency));

    worker_polling_loop(pool, provider, poll_interval, semaphore).await;
}

/// Continuously polls the DB for new jobs and spawns tasks to work on them.
/// Uses a semaphore to limit the maximum number of concurrent tasks.
async fn worker_polling_loop<P>(pool: DbPool, provider: Arc<P>, poll_interval: Duration, semaphore: Arc<Semaphore>)
where
    P: LlmProvider + 'static,
{
    loop {
        match next_job_in_queue(&pool, semaphore.clone()).await {
            Ok(job) => {
                #[allow(clippy::let_underscore_future)]
                let _ = tokio::spawn({
                    let pool = pool.clone();
                    let provider = provider.clone();
                    async move {
                        tracing::info!("Received job {} ({:?}) on website '{}'", job.job_id, job.kind, job.url);
                        let result = handle_job(provider.as_ref(), &job).await;
                        let is_ok = matches!(result, JobResult::Success { .. });
                        match handle_result(&pool, &job, result).await {
                            Ok(ok) => ok,
                            Err(error) => {
                                tracing::error!(
                                    "[SKIP] Failed to handle result for job {} ({:?} - '{}'). Result was ok?: {} - ERROR: {}",
                                    job.job_id,
                                    job.kind,
                                    job.url,
                                    is_ok,
                                    error
                                );
                            }
                        }
                    }
                });
            }
            Err(error) => match error {
                Error::RecordNotFound => {}
                _ => {
                    tracing::error!("[SKIP] Error getting next job from DB queue: {}", error);
                }
            },
        }
        tracing::debug!("Waiting to poll for next job");
        tokio::time::sleep(poll_interval).await;
    }
}
