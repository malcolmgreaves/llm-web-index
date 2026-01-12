use std::sync::Arc;

use core_ltx::{
    TimeUnit, download, get_db_pool, get_poll_interval, is_valid_url,
    llms::{ChatGpt, LlmProvider},
    setup_logging,
};
use data_model_ltx::{
    db,
    models::{JobKindData, JobState, JobStatus, LlmsTxt, LlmsTxtResult},
    schema,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

use worker_ltx::Error;

/// Result of job processing that preserves HTML through error paths
enum JobResult {
    /// Both HTML download and llms.txt generation succeeded
    Success { html: String, llms_txt: core_ltx::LlmsTxt },
    /// HTML downloaded successfully but llms.txt generation failed
    GenerationFailed { html: String, error: Error },
    /// HTML download failed (no HTML to store)
    DownloadFailed { error: Error },
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file., if it exists
    dotenvy::dotenv().ok();

    setup_logging("worker_ltx=debug");

    let provider: Arc<ChatGpt> = Arc::new(ChatGpt::default());

    let pool = get_db_pool();

    let poll_interval = get_poll_interval(TimeUnit::Milliseconds, "WORKER_POLL_INTERVAL_MS", 600);

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
        tokio::time::sleep(poll_interval.clone()).await;
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

/// Downloads HTML and attempts to generate llms.txt.
/// Returns JobResult to preserve HTML even on generation failure.
async fn handle_job<P: LlmProvider>(provider: &P, job: &JobState) -> JobResult {
    // Validate URL
    let url = match is_valid_url(&job.url) {
        Ok(u) => u,
        Err(e) => return JobResult::DownloadFailed { error: e.into() },
    };
    tracing::debug!("[job: {}] Valid URL: {}", job.job_id, url);

    // Download HTML - if this fails, return immediately
    let html = match download(&url).await {
        Ok(h) => h,
        Err(e) => return JobResult::DownloadFailed { error: e.into() },
    };
    tracing::debug!("[job: {}] Downloaded HTML ({} bytes)", job.job_id, html.len());

    // Generate or update llms.txt - if this fails, we still have HTML
    let llms_txt_result = match job.to_kind_data() {
        JobKindData::New => generate_llms_txt(provider, &html).await,
        JobKindData::Update { llms_txt: old_llms_txt } => update_llms_txt(provider, &old_llms_txt, &html).await,
    };

    match llms_txt_result {
        Ok(llms_txt) => {
            tracing::debug!("[job: {}] Generated llms.txt", job.job_id);
            JobResult::Success { html, llms_txt }
        }
        Err(e) => {
            tracing::warn!("[job: {}] Failed to generate llms.txt: {}", job.job_id, e);
            JobResult::GenerationFailed { html, error: e.into() }
        }
    }
}

/// Inserts the result into the llms_txt table & updates job_state appropriately.
/// Handles three cases: success, generation failure (with HTML), download failure (no HTML).
async fn handle_result(pool: &db::DbPool, job: &JobState, result: JobResult) -> Result<(), Error> {
    let mut conn = pool.get().await?;

    match result {
        JobResult::Success { html, llms_txt } => {
            tracing::info!(
                "[job: {}] Successfully produced llms.txt ({:?} - '{}')",
                job.job_id,
                job.kind,
                job.url
            );

            let llms_txt_record = LlmsTxt::from_result(
                job.job_id,
                job.url.clone(),
                LlmsTxtResult::Ok {
                    llms_txt: llms_txt.md_content(),
                },
                html,
            );

            conn.transaction::<_, diesel::result::Error, _>(|mut conn| {
                Box::pin(async move {
                    diesel::insert_into(schema::llms_txt::table)
                        .values(&llms_txt_record)
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

        JobResult::GenerationFailed { html, error } => {
            tracing::error!(
                "[job: {}] Failed to generate llms.txt ({:?} - '{}') Error: {}",
                job.job_id,
                job.kind,
                job.url,
                error
            );

            let llms_txt_record = LlmsTxt::from_result(
                job.job_id,
                job.url.clone(),
                LlmsTxtResult::Error {
                    failure_reason: error.to_string(),
                },
                html,
            );

            conn.transaction::<_, diesel::result::Error, _>(|mut conn| {
                Box::pin(async move {
                    diesel::insert_into(schema::llms_txt::table)
                        .values(&llms_txt_record)
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

            tracing::debug!("[job: {}] Updated DB with failure", job.job_id);
            Ok(())
        }

        JobResult::DownloadFailed { error } => {
            tracing::error!(
                "[job: {}] Failed to download HTML ({:?} - '{}') Error: {}",
                job.job_id,
                job.kind,
                job.url,
                error
            );

            // No llms_txt record - no HTML to store
            // Only mark job as failed in job_state table
            conn.transaction::<_, diesel::result::Error, _>(|mut conn| {
                Box::pin(async move {
                    diesel::update(schema::job_state::table.find(job.job_id))
                        .set(schema::job_state::status.eq(JobStatus::Failure))
                        .execute(&mut conn)
                        .await?;

                    Ok(())
                })
            })
            .await?;

            tracing::debug!("[job: {}] Marked job as failed (no HTML)", job.job_id);
            Ok(())
        }
    }
}
