use std::collections::HashMap;

use data_model_ltx::{
    db,
    models::{JobKind, ResultStatus},
    schema::{job_state, llms_txt},
};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    RecordNotFound,
    DbError(diesel::result::Error),
    DbPoolError(String),
    InvalidUrl(url::ParseError),
    HttpError(reqwest::Error),
    CoreError(core_ltx::Error),
    JobInProgress,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordNotFound => write!(f, "Record not found in database"),
            Self::DbError(e) => write!(f, "Database error: {}", e),
            Self::DbPoolError(s) => write!(f, "Database pool error: {}", s),
            Self::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            Self::HttpError(e) => write!(f, "HTTP error: {}", e),
            Self::CoreError(e) => write!(f, "Core error: {}", e),
            Self::JobInProgress => write!(f, "Job already in progress"),
        }
    }
}

impl std::error::Error for Error {}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::RecordNotFound,
            _ => Self::DbError(error),
        }
    }
}

impl<E: std::fmt::Debug> From<deadpool::managed::PoolError<E>> for Error {
    fn from(error: deadpool::managed::PoolError<E>) -> Self {
        Self::DbPoolError(format!("{:?}", error))
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Self::InvalidUrl(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::HttpError(error)
    }
}

impl From<core_ltx::Error> for Error {
    fn from(error: core_ltx::Error) -> Self {
        Self::CoreError(error)
    }
}

/// Joined result of llms_txt and job_state
#[derive(Debug, Clone, Queryable)]
pub struct LlmsTxtWithKind {
    pub job_id: uuid::Uuid,
    pub url: String,
    pub result_data: String,
    pub result_status: ResultStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub html: String,
    pub kind: JobKind,
}

/// Fetches all completed jobs (Success or Failure) with their llms_txt records
async fn fetch_all_completed_jobs(conn: &mut AsyncPgConnection) -> Result<Vec<LlmsTxtWithKind>, Error> {
    llms_txt::table
        .inner_join(job_state::table.on(llms_txt::job_id.eq(job_state::job_id)))
        .select((
            llms_txt::job_id,
            llms_txt::url,
            llms_txt::result_data,
            llms_txt::result_status,
            llms_txt::created_at,
            llms_txt::html,
            job_state::kind,
        ))
        .order(llms_txt::created_at.desc())
        .load::<LlmsTxtWithKind>(conn)
        .await
        .map_err(Error::from)
}

/// Deduplicates records to get most recent per URL
fn deduplicate_by_url(records: Vec<LlmsTxtWithKind>) -> HashMap<String, LlmsTxtWithKind> {
    let mut url_map: HashMap<String, LlmsTxtWithKind> = HashMap::new();

    for record in records {
        url_map.entry(record.url.clone()).or_insert(record);
    }

    url_map
}

/// Main processing function that polls database and spawns tasks
pub async fn poll_and_process(
    pool: &db::DbPool,
    http_client: &reqwest::Client,
    api_base_url: &str,
) -> Result<usize, Error> {
    let mut conn = pool.get().await?;

    let all_records = fetch_all_completed_jobs(&mut conn).await?;
    let url_records = deduplicate_by_url(all_records);

    tracing::info!("Found {} unique URLs to process", url_records.len());

    let num_urls = url_records.len();

    for (url, record) in url_records {
        let client = http_client.clone();
        let base_url = api_base_url.to_string();

        tokio::spawn(async move {
            match record.result_status {
                ResultStatus::Ok => {
                    if let Err(e) = handle_success(&client, &base_url, &url, &record.html).await {
                        tracing::error!("Error handling success for {}: {}", url, e);
                    }
                }
                ResultStatus::Error => {
                    if let Err(e) = handle_failure(&client, &base_url, &url, record.kind).await {
                        tracing::error!("Error handling failure for {}: {}", url, e);
                    }
                }
            }
        });
    }

    Ok(num_urls)
}

/// Handles successful llms_txt records by checking for HTML changes
async fn handle_success(
    client: &reqwest::Client,
    api_base_url: &str,
    url: &str,
    stored_html: &str,
) -> Result<(), Error> {
    tracing::debug!("Handling success for URL: {}", url);

    let parsed_url = core_ltx::is_valid_url(url)?;
    let fresh_html = core_ltx::download(&parsed_url).await?;
    tracing::debug!("Downloaded {} bytes for {}", fresh_html.len(), url);

    if fresh_html == stored_html {
        tracing::info!("HTML unchanged for {}, skipping update", url);
        return Ok(());
    }

    tracing::info!("HTML changed for {}, sending update request", url);
    send_update_request(client, api_base_url, url).await?;

    Ok(())
}

/// Handles failed llms_txt records by retrying based on job kind
async fn handle_failure(client: &reqwest::Client, api_base_url: &str, url: &str, kind: JobKind) -> Result<(), Error> {
    tracing::debug!("Handling failure for URL: {} (kind: {:?})", url, kind);

    match kind {
        JobKind::New => {
            tracing::info!("Retrying New generation for {}", url);
            send_generate_request(client, api_base_url, url).await?;
        }
        JobKind::Update => {
            tracing::info!("Retrying Update for {}", url);
            send_update_request(client, api_base_url, url).await?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct UrlPayload {
    url: String,
}

#[derive(Deserialize)]
struct JobIdResponse {
    job_id: uuid::Uuid,
}

/// Sends POST /api/llm_txt request to generate new llms.txt
async fn send_generate_request(client: &reqwest::Client, api_base_url: &str, url: &str) -> Result<uuid::Uuid, Error> {
    let endpoint = format!("{}/api/llm_txt", api_base_url);

    let response = client
        .post(&endpoint)
        .json(&UrlPayload { url: url.to_string() })
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::CONFLICT {
        tracing::info!("Job already in progress for {}", url);
        return Err(Error::JobInProgress);
    }

    let job_response: JobIdResponse = response.error_for_status()?.json().await?;

    tracing::info!("Created generate job {} for {}", job_response.job_id, url);
    Ok(job_response.job_id)
}

/// Sends POST /api/update request to update existing llms.txt
async fn send_update_request(client: &reqwest::Client, api_base_url: &str, url: &str) -> Result<uuid::Uuid, Error> {
    let endpoint = format!("{}/api/update", api_base_url);

    let response = client
        .post(&endpoint)
        .json(&UrlPayload { url: url.to_string() })
        .send()
        .await?;

    let job_response: JobIdResponse = response.error_for_status()?.json().await?;

    tracing::info!("Created update job {} for {}", job_response.job_id, url);
    Ok(job_response.job_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_record(
        url: &str,
        created_at: chrono::DateTime<Utc>,
        result_status: ResultStatus,
        kind: JobKind,
    ) -> LlmsTxtWithKind {
        LlmsTxtWithKind {
            job_id: uuid::Uuid::new_v4(),
            url: url.to_string(),
            result_data: "test data".to_string(),
            result_status,
            created_at,
            html: "<html>test</html>".to_string(),
            kind,
        }
    }

    #[test]
    fn test_deduplicate_by_url_keeps_most_recent() {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let two_hours_ago = now - chrono::Duration::hours(2);

        let records = vec![
            create_test_record("https://example.com", now, ResultStatus::Ok, JobKind::New),
            create_test_record("https://example.com", one_hour_ago, ResultStatus::Ok, JobKind::Update),
            create_test_record("https://example.com", two_hours_ago, ResultStatus::Error, JobKind::New),
        ];

        let result = deduplicate_by_url(records);

        assert_eq!(result.len(), 1);
        let record = result.get("https://example.com").unwrap();
        assert_eq!(record.created_at, now);
        assert_eq!(record.result_status, ResultStatus::Ok);
        assert_eq!(record.kind, JobKind::New);
    }

    #[test]
    fn test_deduplicate_by_url_different_urls() {
        let now = Utc::now();

        let records = vec![
            create_test_record("https://example.com", now, ResultStatus::Ok, JobKind::New),
            create_test_record("https://test.com", now, ResultStatus::Ok, JobKind::New),
            create_test_record("https://other.com", now, ResultStatus::Error, JobKind::Update),
        ];

        let result = deduplicate_by_url(records);

        assert_eq!(result.len(), 3);
        assert!(result.contains_key("https://example.com"));
        assert!(result.contains_key("https://test.com"));
        assert!(result.contains_key("https://other.com"));
    }

    #[test]
    fn test_deduplicate_by_url_empty_input() {
        let records = vec![];
        let result = deduplicate_by_url(records);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_deduplicate_by_url_single_record() {
        let now = Utc::now();
        let records = vec![create_test_record(
            "https://example.com",
            now,
            ResultStatus::Ok,
            JobKind::New,
        )];

        let result = deduplicate_by_url(records);

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("https://example.com"));
    }

    #[test]
    fn test_deduplicate_by_url_multiple_urls_with_duplicates() {
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);

        let records = vec![
            create_test_record("https://example.com", now, ResultStatus::Ok, JobKind::New),
            create_test_record("https://example.com", one_hour_ago, ResultStatus::Error, JobKind::New),
            create_test_record("https://test.com", now, ResultStatus::Ok, JobKind::Update),
            create_test_record("https://test.com", one_hour_ago, ResultStatus::Ok, JobKind::New),
        ];

        let result = deduplicate_by_url(records);

        assert_eq!(result.len(), 2);

        let example_record = result.get("https://example.com").unwrap();
        assert_eq!(example_record.created_at, now);
        assert_eq!(example_record.result_status, ResultStatus::Ok);

        let test_record = result.get("https://test.com").unwrap();
        assert_eq!(test_record.created_at, now);
        assert_eq!(test_record.kind, JobKind::Update);
    }

    #[test]
    fn test_error_display() {
        let error = Error::RecordNotFound;
        assert_eq!(error.to_string(), "Record not found in database");

        let error = Error::JobInProgress;
        assert_eq!(error.to_string(), "Job already in progress");

        let error = Error::DbPoolError("connection failed".to_string());
        assert_eq!(error.to_string(), "Database pool error: connection failed");
    }

    #[test]
    fn test_error_from_diesel_not_found() {
        let diesel_error = diesel::result::Error::NotFound;
        let error: Error = diesel_error.into();
        assert!(matches!(error, Error::RecordNotFound));
    }

    #[test]
    fn test_error_from_url_parse_error() {
        let url_result = url::Url::parse("not a valid url");
        assert!(url_result.is_err());

        let url_error = url_result.unwrap_err();
        let error: Error = url_error.into();
        assert!(matches!(error, Error::InvalidUrl(_)));
    }
}
