use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::collections::HashMap;

use crate::models::{
    GetLlmTxtError, JobIdResponse, JobKindData, JobState, JobStatus as JobStatusEnum,
    LlmTxtResponse, LlmsTxt, LlmsTxtListItem, LlmsTxtListResponse, PostLlmTxtError, PutLlmTxtError,
    ResultStatus, UpdateLlmTxtError, UrlPayload,
};
use crate::routes::AppError;
use crate::{db::DbPool, routes::job_state::in_progress_jobs};

/// Gets the most recent llm.txt entry for the website, if available.
///
/// Only returns an Ok result if:
///   - There's at least one row with a result of OK
///   - There's at least one row that has the url
/// If there are multiple, the most recent one (using `created_at`) is returned.
///
/// An Error is returned if there are either no matching rows or if there's an internal DB error.
pub async fn fetch_llms_txt(
    executor: impl sqlx::PgExecutor<'_>,
    url: &str,
) -> Result<LlmsTxt, sqlx::Error> {
    sqlx::query_as!(
        LlmsTxt,
        r#"
        SELECT job_id, url, result_data, result_status AS "result_status: ResultStatus", created_at
        FROM llms_txt
        WHERE url = $1
          AND result_status = $2
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        url,
        ResultStatus::Ok as ResultStatus
    )
    .fetch_one(executor)
    .await
}

/// GET /api/llm_txt - Retrieve llms.txt content for a URL
pub async fn get_llm_txt(
    State(pool): State<DbPool>,
    Query(payload): Query<UrlPayload>,
) -> Result<impl IntoResponse, GetLlmTxtError> {
    match fetch_llms_txt(&pool, &payload.url).await {
        Ok(llms_txt_record) => match llms_txt_record.result_status {
            ResultStatus::Ok => Ok((
                StatusCode::OK,
                Json(LlmTxtResponse {
                    content: llms_txt_record.result_data,
                }),
            )),
            ResultStatus::Error => Err(GetLlmTxtError::GenerationFailure(
                llms_txt_record.result_data,
            )),
        },
        Err(e) => Err(e.into()),
    }
}

/// Create a request to generate a new llms.txt
async fn new_llms_txt_generate_job(
    executor: impl sqlx::PgExecutor<'_>,
    url: &str,
) -> Result<JobIdResponse, sqlx::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(
        job_id,
        url.to_string(),
        JobStatusEnum::Queued,
        JobKindData::New,
    );

    sqlx::query!(
        r#"
        INSERT INTO job_state (job_id, url, status, kind, llms_txt)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        new_job.job_id,
        new_job.url,
        new_job.status as JobStatusEnum,
        new_job.kind as crate::models::JobKind,
        new_job.llms_txt
    )
    .execute(executor)
    .await?;

    Ok(JobIdResponse { job_id })
}

/// POST /api/llm_txt - Create a new job to generate llms.txt
pub async fn post_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PostLlmTxtError> {
    let mut tx = pool.begin().await?;

    match fetch_llms_txt(&mut *tx, &payload.url).await {
        Ok(_) => {
            tx.rollback().await?;
            Err(PostLlmTxtError::AlreadyGenerated)
        }
        Err(e) => match e {
            sqlx::Error::RowNotFound => match in_progress_jobs(&mut *tx, &payload.url).await {
                Ok(existing_jobs) if !existing_jobs.is_empty() => {
                    tx.rollback().await?;
                    Err(PostLlmTxtError::JobsInProgress(existing_jobs))
                }
                Ok(_) | Err(sqlx::Error::RowNotFound) => {
                    let job_id_response = new_llms_txt_generate_job(&mut *tx, &payload.url).await?;
                    tx.commit().await?;
                    Ok((StatusCode::CREATED, Json(job_id_response)))
                }
                Err(e_jobs) => {
                    tx.rollback().await?;
                    Err(e_jobs.into())
                }
            },
            _ => {
                tx.rollback().await?;
                Err(e.into())
            }
        },
    }
}

/// Create a request to update an existing llms.txt
async fn update_llms_txt_generation(
    executor: impl sqlx::PgExecutor<'_>,
    url: &str,
    llms_txt: &str,
) -> Result<JobIdResponse, sqlx::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(
        job_id,
        url.to_string(),
        JobStatusEnum::Queued,
        JobKindData::Update {
            llms_txt: llms_txt.to_string(),
        },
    );

    sqlx::query!(
        r#"
        INSERT INTO job_state (job_id, url, status, kind, llms_txt)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        new_job.job_id,
        new_job.url,
        new_job.status as JobStatusEnum,
        new_job.kind as crate::models::JobKind,
        new_job.llms_txt
    )
    .execute(executor)
    .await?;

    Ok(JobIdResponse { job_id })
}

/// POST /api/update - Create an update job for existing llms.txt
pub async fn post_update(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, UpdateLlmTxtError> {
    let mut tx = pool.begin().await?;

    match fetch_llms_txt(&mut *tx, &payload.url).await {
        Ok(llms_txt) => {
            // Create an update job using the existing llms.txt result_data
            let job_id_response =
                update_llms_txt_generation(&mut *tx, &payload.url, &llms_txt.result_data).await?;
            tx.commit().await?;
            Ok((StatusCode::CREATED, Json(job_id_response)))
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e.into())
        }
    }
}

/// PUT /api/llm_txt - Create a new job: either a 1st time or an update
pub async fn put_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PutLlmTxtError> {
    let mut tx = pool.begin().await?;

    match fetch_llms_txt(&mut *tx, &payload.url).await {
        Ok(llms_txt) => {
            let job_id_response =
                update_llms_txt_generation(&mut *tx, &payload.url, &llms_txt.result_data).await?;
            tx.commit().await?;
            Ok((StatusCode::CREATED, Json(job_id_response)))
        }
        Err(e) => match e {
            sqlx::Error::RowNotFound => {
                let job_id_response = new_llms_txt_generate_job(&mut *tx, &payload.url).await?;
                tx.commit().await?;
                Ok((StatusCode::CREATED, Json(job_id_response)))
            }
            _ => {
                tx.rollback().await?;
                Err(e.into())
            }
        },
    }
}

// GET /api/list - List all successfully fetched llms.txt files
pub async fn get_list(State(pool): State<DbPool>) -> Result<impl IntoResponse, AppError> {
    // Load all Ok records ordered by url and created_at DESC
    let all_records = sqlx::query_as!(
        LlmsTxt,
        r#"
        SELECT job_id, url, result_data, result_status AS "result_status: ResultStatus", created_at
        FROM llms_txt
        WHERE result_status = $1
        ORDER BY url ASC, created_at DESC
        "#,
        ResultStatus::Ok as ResultStatus
    )
    .fetch_all(&pool)
    .await?;

    // Deduplicate by URL, keeping only the most recent
    let mut url_map: HashMap<String, String> = HashMap::new();
    for record in all_records {
        url_map.entry(record.url).or_insert(record.result_data);
    }

    // Convert to list response
    let items: Vec<LlmsTxtListItem> = url_map
        .into_iter()
        .map(|(url, llm_txt)| LlmsTxtListItem { url, llm_txt })
        .collect();

    Ok((StatusCode::OK, Json(LlmsTxtListResponse { items })))
}
