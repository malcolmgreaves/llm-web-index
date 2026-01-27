use std::collections::HashMap;
use std::sync::Arc;

use core_ltx::{db, normalize_html, web_html::compute_html_checksum};
use data_model_ltx::{
    models::{JobKind, ResultStatus},
    schema::{job_state, llms_txt},
};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::AuthenticatedClient;
use crate::LlmsTxtWithKind;
use crate::errors::Error;

/// Gets the most recent llms.txt for each url and spawns a task to determine if the llms.txt should be updated/regenerated.
pub async fn poll_and_process(
    pool: &db::DbPool,
    http_client: &std::sync::Arc<AuthenticatedClient>,
    api_base_url: &str,
) -> Result<usize, Error> {
    let url_records = most_recent_completed(pool).await?;
    let num_urls = url_records.len();
    tracing::info!("Found {} unique URLs to process.", num_urls);

    handle_record_updates(http_client, api_base_url, url_records).await;

    Ok(num_urls)
}

/// Gets only the most recent llms.txt record for each URL in the DB.
async fn most_recent_completed(pool: &db::DbPool) -> Result<HashMap<String, LlmsTxtWithKind>, Error> {
    let mut conn = pool.get().await?;
    let all_records = fetch_all_completed_jobs(&mut conn).await?;
    let url_records = deduplicate_by_url(all_records);
    Ok(url_records)
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
            llms_txt::html_compress,
            llms_txt::html_checksum,
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

/// Handles all llms.txt records by either attempting to regenerate (for a failed row) or update (for a success) the llms.txt.
async fn handle_record_updates(
    http_client: &std::sync::Arc<AuthenticatedClient>,
    api_base_url: &str,
    url_records: HashMap<String, LlmsTxtWithKind>,
) {
    for (url, record) in url_records {
        tokio::spawn({
            let http_client = http_client.clone();
            let api_base_url = api_base_url.to_string();
            async move {
                match record.result_status {
                    ResultStatus::Ok => {
                        if let Err(e) = handle_success(&http_client, &api_base_url, &url, &record.html_checksum).await {
                            tracing::error!("Error handling success for {}: {}", url, e);
                        }
                    }
                    ResultStatus::Error => {
                        if let Err(e) = handle_failure(&http_client, &api_base_url, &url, record.kind).await {
                            tracing::error!("Error handling failure for {}: {}", url, e);
                        }
                    }
                }
            }
        });
    }
}

/// Sends llms.txt update request to API server if the website's HTML has changed.
async fn handle_success(
    client: &Arc<AuthenticatedClient>,
    api_base_url: &str,
    url: &str,
    stored_checksum: &str,
) -> Result<(), Error> {
    tracing::debug!("Handling success for URL: '{}'", url);

    let parsed_url = core_ltx::is_valid_url(url)?;
    let fresh_html = core_ltx::download(&parsed_url).await?;
    tracing::debug!("Downloaded {} bytes for '{}'", fresh_html.len(), url);

    // Compute checksum of freshly downloaded HTML
    let normalized_fresh_html = normalize_html(&fresh_html)?;
    let fresh_checksum = compute_html_checksum(&normalized_fresh_html)?;

    if fresh_checksum == stored_checksum {
        tracing::info!(
            "HTML unchanged (checksum: {}) for '{}', skipping update.",
            stored_checksum,
            url
        );
        return Ok(());
    }

    tracing::info!(
        "HTML changed for '{}' (checksum: {} -> {}), sending update request.",
        url,
        stored_checksum,
        fresh_checksum
    );
    let job_id = send_update_request(client, api_base_url, url).await?;
    tracing::info!("Confirmed: Job ID {} for update on '{}'", job_id, url);

    Ok(())
}

/// Sends request to API server to regenerate llms.txt since it failed to generate it last time.
async fn handle_failure(
    client: &Arc<AuthenticatedClient>,
    api_base_url: &str,
    url: &str,
    kind: JobKind,
) -> Result<(), Error> {
    tracing::debug!("Handling failure for URL: '{}' ({:?})", url, kind);

    let job_id = match kind {
        JobKind::New => {
            tracing::info!("Retrying New generation for '{}'", url);
            send_generate_request(client, api_base_url, url).await?
        }
        JobKind::Update => {
            tracing::info!("Retrying Update for '{}'", url);
            send_update_request(client, api_base_url, url).await?
        }
    };
    tracing::info!("Confirmed: Job ID {} ({:?}) for '{}'", job_id, kind, url);

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
async fn send_generate_request(
    client: &Arc<AuthenticatedClient>,
    _api_base_url: &str,
    url: &str,
) -> Result<uuid::Uuid, Error> {
    tracing::debug!("API request: POST /api/llm_txt");
    let payload = UrlPayload { url: url.to_string() };
    let response = client.post("/api/llm_txt", &payload).await?;
    tracing::debug!("received response from API server");

    // if response.status() == reqwest::StatusCode::CONFLICT {
    //     tracing::info!("Job already in progress for '{}'", url);
    //     return Err(Error::JobInProgress);
    // }

    let job_response: JobIdResponse = response.error_for_status()?.json().await?;
    tracing::info!("Created generate job {} for '{}'", job_response.job_id, url);
    Ok(job_response.job_id)
}

/// Sends POST /api/update request to update existing llms.txt
async fn send_update_request(
    client: &Arc<AuthenticatedClient>,
    _api_base_url: &str,
    url: &str,
) -> Result<uuid::Uuid, Error> {
    tracing::debug!("API request: POST /api/update");
    let payload = UrlPayload { url: url.to_string() };
    let response = client.post("/api/update", &payload).await?;
    tracing::debug!("received response from API server");

    let job_response: JobIdResponse = response.error_for_status()?.json().await?;
    tracing::info!("Created update job {} for '{}'", job_response.job_id, url);
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
        let html = "<html>test</html>";
        let normalized_fresh_html = normalize_html(&html).unwrap();
        let html_checksum = compute_html_checksum(&normalized_fresh_html).unwrap();
        let html_compress = core_ltx::compress_string(html).unwrap();

        LlmsTxtWithKind {
            job_id: uuid::Uuid::new_v4(),
            url: url.to_string(),
            result_data: "test data".to_string(),
            result_status,
            created_at,
            html_compress,
            html_checksum,
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
}
