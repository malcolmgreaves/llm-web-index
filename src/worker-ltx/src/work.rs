use std::sync::Arc;

use core_ltx::{
    download, is_valid_url,
    llms::{LlmProvider, generate_llms_txt, update_llms_txt},
};

use core_ltx::db;
use data_model_ltx::{
    models::{JobKindData, JobState, JobStatus, LlmsTxt, LlmsTxtResult},
    schema,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::errors::Error;

/// Result of job processing that preserves HTML through error paths
pub enum JobResult {
    /// Both HTML download and llms.txt generation succeeded
    Success { html: String, llms_txt: core_ltx::LlmsTxt },
    /// HTML downloaded successfully but llms.txt generation failed
    GenerationFailed { html: String, error: Error },
    /// HTML download failed (no HTML to store)
    DownloadFailed { error: Error },
}

/// Query the DB for a job to be performed.
/// The semaphore controls the maximum number of concurrent jobs that the worker can handle.
pub async fn next_job_in_queue(
    pool: &db::DbPool,
    semaphore: Arc<Semaphore>,
) -> Result<(JobState, OwnedSemaphorePermit), Error> {
    let mut conn = pool.get().await?;

    let job_permit: (JobState, OwnedSemaphorePermit) = conn
        .transaction::<_, Error, _>(|conn| {
            Box::pin(async move {
                // Acquire a permit before spawning the task.
                // This will block if we've reached max_concurrency, effectively queuing tasks.
                tracing::debug!("Acquiring semaphore before checking for new job to acquire.");
                let permit = semaphore.clone().acquire_owned().await?;
                tracing::debug!("Semaphore permit acquired. Querying DB for jobs.");
                // NOTE: If we return an Err, we will drop the permit, allowing another job to be worked on.
                //       We only pass the acquired semaphore permit if we get a job to work on.

                // Query for a job with status Queued or Started using FOR UPDATE SKIP LOCKED.
                // => This ensures multiple workers can safely claim jobs without conflicts.
                let job: JobState = schema::job_state::table
                    .filter(
                        schema::job_state::status
                            .eq(JobStatus::Queued)
                            .or(schema::job_state::status.eq(JobStatus::Started)),
                    )
                    .for_update()
                    .skip_locked()
                    // TODO: add a created_at field to job_state and order on this first, then order by job ID for fully consistent
                    //       ordering + ensure that jobs are processed FIFO
                    .order(schema::job_state::job_id.asc()) // provide consistent ordering for de-queueing jobs
                    .first::<JobState>(conn)
                    .await?;

                // if we have such a job, make sure we mark it as running as this worker has claimed it
                diesel::update(schema::job_state::table.find(job.job_id))
                    .set(schema::job_state::status.eq(JobStatus::Running))
                    .execute(conn)
                    .await?;

                // Make sure our job reflects this `status` update!
                let job = {
                    let mut job = job;
                    job.status = JobStatus::Running;
                    job
                };

                Ok((job, permit))
            })
        })
        .await?;

    Ok(job_permit)
}

/// Downloads HTML and attempts to generate llms.txt.
/// Returns JobResult to preserve HTML even on generation failure.
pub async fn handle_job<P: LlmProvider>(provider: &P, job: &JobState) -> JobResult {
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
pub async fn handle_result(pool: &db::DbPool, job: &JobState, result: JobResult) -> Result<(), Error> {
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
