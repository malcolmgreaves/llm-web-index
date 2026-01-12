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

use worker_ltx::Error;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "worker_ltx=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    let provider: Arc<ChatGpt> = Arc::new(ChatGpt::default());

    let pool = db::establish_connection_pool(&database_url)
        .unwrap_or_else(|_| panic!("Couldn't connect to database: {}", database_url));

    let poll_interval = {
        let poll_interval_ms = std::env::var("WORKER_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| "600".to_string())
            .parse::<u64>()
            .expect("WORKER_POLL_INTERVAL_MS must be a valid number");

        tracing::info!("Worker started, polling every {}ms", poll_interval_ms);
        Duration::from_millis(poll_interval_ms)
    };

    // Worker polling loop
    loop {
        match next_job_in_queue(&pool).await {
            Ok(job) => {
                let _ = tokio::spawn({
                    let pool = pool.clone();
                    let provider = provider.clone();
                    async move {
                        tracing::info!("Received job {} - {:?} on website '{}'", job.job_id, job.kind, job.url);
                        let result = handle_job(provider.as_ref(), &job).await;
                        let is_ok = result.is_ok();
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
            Err(error) => {
                match error {
                    Error::RecordNotFound => {}
                    _ => {
                        tracing::error!("[SKIP] Error getting next job from DB queue: {}", error);
                    }
                }
                tokio::time::sleep(poll_interval.clone()).await;
            }
        }
    }
}

async fn next_job_in_queue(pool: &db::DbPool) -> Result<JobState, Error> {
    let mut conn = pool.get().await?;

    let job: JobState = conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Query for a job with status Queued or Started using FOR UPDATE SKIP LOCKED
                // This ensures multiple workers can safely claim jobs without conflicts
                let job: JobState = schema::job_state::table
                    .filter(
                        schema::job_state::status
                            .eq(JobStatus::Queued)
                            .or(schema::job_state::status.eq(JobStatus::Started)),
                    )
                    .order(schema::job_state::job_id.asc()) // Process jobs in order
                    .for_update()
                    .skip_locked()
                    .first::<JobState>(conn)
                    .await?;

                // if we have such a job, make sure we mark it as running as this worker has claimed it
                diesel::update(schema::job_state::table.find(job.job_id))
                    .set(schema::job_state::status.eq(JobStatus::Running))
                    .execute(conn)
                    .await?;

                Ok(job)
            })
        })
        .await?;

    Ok(job)
}

use core_ltx::llms::{generate_llms_txt, update_llms_txt};

/// Creates a new llms.txt or updates an old one, depending on the job kind.
async fn handle_job<P: LlmProvider>(provider: &P, job: &JobState) -> Result<core_ltx::LlmsTxt, Error> {
    let url = is_valid_url(&job.url)?;
    tracing::debug!("[job: {}] Valid URL: {}", job.job_id, url);

    let html = download(&url).await?;
    tracing::debug!("[job: {}] Downloaded HTML", job.job_id);

    let llms_txt = match job.to_kind_data() {
        JobKindData::New => generate_llms_txt(provider, &html).await?,
        JobKindData::Update { llms_txt: old_llms_txt } => update_llms_txt(provider, &old_llms_txt, &html).await?,
    };
    tracing::debug!("[job: {}] Generated new llms.txt", job.job_id);

    Ok(llms_txt)
}

/// Inserts the result (valid or error) into the llms.txt table & updates job_state table appropriately.
async fn handle_result(
    pool: &db::DbPool,
    job: &JobState,
    result: Result<core_ltx::LlmsTxt, Error>,
) -> Result<(), Error> {
    let mut conn = pool.get().await?;

    match result {
        Ok(llms_txt_content) => {
            tracing::info!(
                "[job: {}] Successfully produced llms.txt ({:?} - '{}')",
                job.job_id,
                job.kind,
                job.url
            );

            let llms_txt = LlmsTxt::from_result(
                job.job_id,
                job.url.clone(),
                LlmsTxtResult::Ok {
                    llms_txt: llms_txt_content.md_content(),
                },
            );

            conn.transaction::<_, diesel::result::Error, _>(|mut conn| {
                Box::pin(async move {
                    diesel::insert_into(schema::llms_txt::table)
                        .values(&llms_txt)
                        .execute(&mut conn)
                        .await?;

                    diesel::update(schema::job_state::table.find(job.job_id))
                        .set(schema::job_state::status.eq(JobStatus::Success))
                        .execute(&mut conn)
                        .await?;

                    Ok(())
                })
            })
            .await?;

            tracing::debug!("[job: {}] Updated DB", job.job_id);

            Ok(())
        }

        Err(e) => {
            tracing::error!(
                "[job: {}] Failed to produce llms.txt ({:?} - '{}') Error: {}",
                job.job_id,
                job.kind,
                job.url,
                e
            );

            let llms_txt = LlmsTxt::from_result(
                job.job_id,
                job.url.clone(),
                LlmsTxtResult::Error {
                    failure_reason: e.to_string(),
                },
            );

            conn.transaction::<_, diesel::result::Error, _>(|mut conn| {
                Box::pin(async move {
                    diesel::insert_into(schema::llms_txt::table)
                        .values(&llms_txt)
                        .execute(&mut conn)
                        .await?;

                    diesel::update(schema::job_state::table.find(job.job_id))
                        .set(schema::job_state::status.eq(JobStatus::Failure))
                        .execute(&mut conn)
                        .await?;

                    Ok(())
                })
            })
            .await?;

            tracing::debug!("[job: {}] Updated DB", job.job_id);

            Ok(())
        }
    }
}
