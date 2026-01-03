use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::prelude::*;

use crate::models::{
    GetLlmTxtError, JobIdResponse, JobKindData, JobState, JobStatus as JobStatusEnum,
    LlmTxtResponse, LlmsTxt, LlmsTxtListItem, LlmsTxtListResponse, PostLlmTxtError, PutLlmTxtError,
    ResultStatus, UpdateLlmTxtError, UrlPayload,
};
use crate::routes::AppError;
use crate::schema::{job_state, llms_txt};
use crate::{
    db::{Conn, DbPool},
    routes::job_state::in_progress_jobs,
};

/// Gets the most recent llm.txt entry for the website, if available.
pub fn fetch_llms_txt(conn: &mut Conn, url: &str) -> Result<LlmsTxt, diesel::result::Error> {
    llms_txt::table
        .filter(llms_txt::url.eq(url))
        .order(llms_txt::created_at.desc())
        .select(LlmsTxt::as_select())
        .first(conn)
}

/// GET /api/llm_txt - Retrieve llms.txt content for a URL
pub async fn get_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, GetLlmTxtError> {
    let mut conn = pool.get()?;

    match fetch_llms_txt(&mut conn, &payload.url) {
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
fn new_llms_txt_generate_job(
    conn: &mut Conn,
    url: &str,
) -> Result<JobIdResponse, diesel::result::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(
        job_id,
        url.to_string(),
        JobStatusEnum::Queued,
        JobKindData::New,
    );

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(conn)?;

    Ok(JobIdResponse { job_id })
}

/// POST /api/llm_txt - Create a new job to generate llms.txt
pub async fn post_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PostLlmTxtError> {
    pool.get()?
        .transaction(|conn| match fetch_llms_txt(conn, &payload.url) {
            Ok(_) => Err(PostLlmTxtError::AlreadyGenerated),
            Err(e) => match e {
                diesel::result::Error::NotFound => match in_progress_jobs(conn, &payload.url) {
                    Ok(existing_jobs) => Err(PostLlmTxtError::JobsInProgress(existing_jobs)),

                    Err(e_jobs) => match e_jobs {
                        diesel::result::Error::NotFound => {
                            let job_id_response = new_llms_txt_generate_job(conn, &payload.url)?;
                            Ok((StatusCode::CREATED, Json(job_id_response)))
                        }

                        _ => Err(e_jobs.into()),
                    },
                },
                _ => Err(e.into()),
            },
        })
}

/// Create a request to update an existing llms.txt
fn update_llms_txt_generation(
    conn: &mut Conn,
    url: &str,
    llms_txt: &str,
) -> Result<JobIdResponse, diesel::result::Error> {
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(
        job_id,
        url.to_string(),
        JobStatusEnum::Queued,
        JobKindData::Update {
            llms_txt: llms_txt.to_string(),
        },
    );

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(conn)?;

    Ok(JobIdResponse { job_id })
}

// POST /api/update - Create an update job for existing llms.txt
pub async fn post_update(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, UpdateLlmTxtError> {
    pool.get()?.transaction(|conn| {
        match fetch_llms_txt(conn, &payload.url) {
            Ok(llms_txt) => {
                // Create an update job using the existing llms.txt result_data
                let job_id_response =
                    update_llms_txt_generation(conn, &payload.url, &llms_txt.result_data)?;
                Ok((StatusCode::CREATED, Json(job_id_response)))
            }

            Err(e) => Err(e.into()),
        }
    })
}

/// PUT /api/llm_txt - Create a new job: either a 1st time or an update
pub async fn put_llm_txt(
    State(pool): State<DbPool>,
    Json(payload): Json<UrlPayload>,
) -> Result<impl IntoResponse, PutLlmTxtError> {
    let mut conn = pool.get()?;

    match fetch_llms_txt(&mut conn, &payload.url) {
        Ok(llms_txt) => {}
    }

    // Determine job kind and status code based on existing data
    let (kind_data, status_code) = match existing {
        Some(llms_txt_record) if llms_txt_record.result_status == ResultStatus::Ok => {
            // If there's an Ok result, create an Update job (like POST /api/update)
            (
                JobKindData::Update {
                    llms_txt: llms_txt_record.result_data,
                },
                StatusCode::OK,
            )
        }
        _ => {
            // Otherwise, create a New job (like POST /api/llm_txt)
            // This covers: no existing entry OR existing entry with Error status
            (JobKindData::New, StatusCode::CREATED)
        }
    };

    // Create the job
    let job_id = uuid::Uuid::new_v4();
    let new_job = JobState::from_kind_data(job_id, payload.url, JobStatusEnum::Queued, kind_data);

    diesel::insert_into(job_state::table)
        .values(&new_job)
        .execute(&mut conn)
        .map_err(|_| PutLlmTxtError::Unknown)?;

    Ok((status_code, Json(JobIdResponse { job_id })))
}

// GET /api/list - List all successfully fetched llms.txt files
pub async fn get_list(State(pool): State<DbPool>) -> Result<impl IntoResponse, AppError> {
    use std::collections::HashMap;

    let mut conn = pool.get()?;

    // Load all Ok records ordered by url and created_at DESC
    let all_records = llms_txt::table
        .filter(llms_txt::result_status.eq(ResultStatus::Ok))
        .order((llms_txt::url.asc(), llms_txt::created_at.desc()))
        .select(LlmsTxt::as_select())
        .load::<LlmsTxt>(&mut conn)?;

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
