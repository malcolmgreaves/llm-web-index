use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use std::collections::HashMap;

use data_model_ltx::db::DbPool;
use data_model_ltx::models::{
    AppError, GetLlmTxtError, JobIdResponse, JobKindData, JobState, JobStatus, LlmTxtResponse, LlmsTxt,
    LlmsTxtListItem, LlmsTxtListResponse, PostLlmTxtError, PutLlmTxtError, ResultStatus, UpdateLlmTxtError, UrlPayload,
};
use data_model_ltx::schema::{job_state, llms_txt};

use crate::routes::job_state::in_progress_jobs;

/// Gets the most recent llm.txt entry for the website, if available.
///
/// Only returns an Ok result if:
///   - There's at least one row with a result of OK
///   - There's at least one row that has the url
///
/// If there are multiple, the most recent one (using `created_at`) is returned.
///
/// An Error is returned if there are either no matching rows or if there's an internal DB error.
pub async fn fetch_llms_txt(conn: &mut AsyncPgConnection, url: &str) -> Result<LlmsTxt, diesel::result::Error> {
    llms_txt::table
        .filter(llms_txt::url.eq(url))
        .filter(llms_txt::result_status.eq(ResultStatus::Ok))
        .order(llms_txt::created_at.desc())
        .select(LlmsTxt::as_select())
        .first(conn)
        .await
}

/// GET /api/llm_txt - Retrieve llms.txt content for a URL
pub async fn get_llm_txt(
    State(pool): State<DbPool>,
    Query(payload): Query<UrlPayload>,
) -> Result<impl IntoResponse, GetLlmTxtError> {
    let mut conn = pool.get().await?;

    match fetch_llms_txt(&mut conn, &payload.url).await {
        Ok(llms_txt_record) => {
            tracing::debug!("{} is Ok: {:?}", payload.url, llms_txt_record);
            match llms_txt_record.result_status {
                ResultStatus::Ok => Ok((
                    StatusCode::OK,
                    Json(LlmTxtResponse {
                        content: llms_txt_record.result_data,
                    }),
                )),
                ResultStatus::Error => Err(GetLlmTxtError::GenerationFailure(llms_txt_record.result_data)),
            }
        }
        Err(e) => {
            tracing::debug!("{} is Error: {}", payload.url, e);
            Err(e.into())
        }
    }
}

/// Create a request to generate a new llms.txt
async fn new_llms_txt_generate_job(
    conn: &mut AsyncPgConnection,
    url: &str,
) -> Result<JobIdResponse, diesel::result::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(job_id, url.to_string(), JobStatus::Queued, JobKindData::New);

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(conn)
        .await?;

    Ok(JobIdResponse { job_id })
}

/// POST /api/llm_txt - Create a new job to generate llms.txt
pub async fn post_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PostLlmTxtError> {
    let mut conn = pool.get().await?;
    conn.transaction(|conn| {
        async move {
            match fetch_llms_txt(conn, &payload.url).await {
                Ok(x) => {
                    tracing::debug!("{} is Ok: {:?}", payload.url, x);
                    Err(PostLlmTxtError::AlreadyGenerated)
                }
                Err(e) => match e {
                    diesel::result::Error::NotFound => match in_progress_jobs(conn, &payload.url).await {
                        Ok(existing_jobs) => {
                            tracing::debug!("{} has existing jobs: {:?}", payload.url, existing_jobs);
                            Err(PostLlmTxtError::JobsInProgress(existing_jobs))
                        }

                        Err(e_jobs) => match e_jobs {
                            diesel::result::Error::NotFound => {
                                tracing::debug!("{} not found", payload.url);
                                let job_id_response = new_llms_txt_generate_job(conn, &payload.url).await?;
                                Ok((StatusCode::CREATED, Json(job_id_response)))
                            }
                            _ => {
                                tracing::debug!("{} not found -- other error: {}", payload.url, e_jobs);
                                Err(e_jobs.into())
                            }
                        },
                    },
                    _ => {
                        tracing::debug!("{} error fetching llms_txt: {}", payload.url, e);
                        Err(e.into())
                    }
                },
            }
        }
        .scope_boxed()
    })
    .await
}

/// Create a request to update an existing llms.txt
async fn update_llms_txt_generation(
    conn: &mut AsyncPgConnection,
    url: &str,
    llms_txt: &str,
) -> Result<JobIdResponse, diesel::result::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(
        job_id,
        url.to_string(),
        JobStatus::Queued,
        JobKindData::Update {
            llms_txt: llms_txt.to_string(),
        },
    );

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(conn)
        .await?;

    Ok(JobIdResponse { job_id })
}

/// POST /api/update - Create an update job for existing llms.txt
pub async fn post_update(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, UpdateLlmTxtError> {
    let mut conn = pool.get().await?;
    conn.transaction(|conn| {
        async move {
            match fetch_llms_txt(conn, &payload.url).await {
                Ok(llms_txt) => {
                    // Create an update job using the existing llms.txt result_data
                    let job_id_response = update_llms_txt_generation(conn, &payload.url, &llms_txt.result_data).await?;
                    Ok((StatusCode::CREATED, Json(job_id_response)))
                }

                Err(e) => Err(e.into()),
            }
        }
        .scope_boxed()
    })
    .await
}

/// PUT /api/llm_txt - Create a new job: either a 1st time or an update
pub async fn put_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PutLlmTxtError> {
    let mut conn = pool.get().await?;
    conn.transaction(|conn| {
        async move {
            match fetch_llms_txt(conn, &payload.url).await {
                Ok(llms_txt) => {
                    let job_id_response = update_llms_txt_generation(conn, &payload.url, &llms_txt.result_data).await?;
                    Ok((StatusCode::CREATED, Json(job_id_response)))
                }

                Err(e) => match e {
                    diesel::result::Error::NotFound => {
                        let job_id_response = new_llms_txt_generate_job(conn, &payload.url).await?;
                        Ok((StatusCode::CREATED, Json(job_id_response)))
                    }
                    _ => Err(e.into()),
                },
            }
        }
        .scope_boxed()
    })
    .await
}

// GET /api/list - List all successfully fetched llms.txt files
pub async fn get_list(State(pool): State<DbPool>) -> Result<impl IntoResponse, AppError> {
    let mut conn = pool.get().await?;

    // Load all Ok records ordered by url and created_at DESC
    let all_records = llms_txt::table
        .filter(llms_txt::result_status.eq(ResultStatus::Ok))
        .order((llms_txt::url.asc(), llms_txt::created_at.desc()))
        .select(LlmsTxt::as_select())
        .load::<LlmsTxt>(&mut conn)
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
