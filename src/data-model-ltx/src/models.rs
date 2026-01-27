use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SqlType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use uuid::Uuid;

use core_ltx::db::PoolError;

// SQL type definitions for custom enums
// Note: These types use snake_case to match PostgreSQL type names
#[allow(non_camel_case_types)]
#[derive(SqlType, diesel::query_builder::QueryId, Debug, Clone, Copy)]
#[diesel(postgres_type(name = "job_status"))]
pub struct Job_status;

#[allow(non_camel_case_types)]
#[derive(SqlType, diesel::query_builder::QueryId, Debug, Clone, Copy)]
#[diesel(postgres_type(name = "job_kind"))]
pub struct Job_kind;

#[allow(non_camel_case_types)]
#[derive(SqlType, diesel::query_builder::QueryId, Debug, Clone, Copy)]
#[diesel(postgres_type(name = "result_status"))]
pub struct Result_status;

// JobStatus enum
/// Status of a job in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Job_status)]
pub enum JobStatus {
    /// A newly created job
    Queued,
    /// Job manager started job
    Started,
    /// Worker received job
    Running,
    /// New or updated llms.txt file made and added to database
    Success,
    /// Worker failed
    Failure,
}

impl JobStatus {
    // True if job's status is Success or Failure. False means it's Queued, Started, or Running.
    pub fn is_completed(&self) -> bool {
        match self {
            Self::Queued | Self::Started | Self::Running => false,
            Self::Success | Self::Failure => true,
        }
    }
}

impl ToSql<Job_status, Pg> for JobStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            JobStatus::Queued => "queued",
            JobStatus::Started => "started",
            JobStatus::Running => "running",
            JobStatus::Success => "success",
            JobStatus::Failure => "failure",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Job_status, Pg> for JobStatus {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"queued" => Ok(JobStatus::Queued),
            b"started" => Ok(JobStatus::Started),
            b"running" => Ok(JobStatus::Running),
            b"success" => Ok(JobStatus::Success),
            b"failure" => Ok(JobStatus::Failure),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// JobKind enum
/// Type of job operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Job_kind)]
pub enum JobKind {
    /// New llms.txt fetch
    New,
    /// Update existing llms.txt
    Update,
}

impl ToSql<Job_kind, Pg> for JobKind {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            JobKind::New => "new",
            JobKind::Update => "update",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Job_kind, Pg> for JobKind {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"new" => Ok(JobKind::New),
            b"update" => Ok(JobKind::Update),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// ResultStatus enum
/// Status of an llms.txt fetch result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Result_status)]
pub enum ResultStatus {
    /// Successfully fetched llms.txt
    Ok,
    /// Failed to fetch llms.txt
    Error,
}

impl ToSql<Result_status, Pg> for ResultStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            ResultStatus::Ok => "ok",
            ResultStatus::Error => "error",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Result_status, Pg> for ResultStatus {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"ok" => Ok(ResultStatus::Ok),
            b"error" => Ok(ResultStatus::Error),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// job_state table model (database representation)
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::job_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct JobState {
    pub job_id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub kind: JobKind,
    pub llms_txt: Option<String>,
}

// JobKindData - ergonomic Rust enum for the job kind
/// Kind of job operation with associated data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum JobKindData {
    /// New llms.txt fetch
    New,
    /// Update existing llms.txt with prior content
    Update { llms_txt: String },
}

impl JobState {
    /// Convert database representation to ergonomic JobKindData enum
    pub fn to_kind_data(&self) -> JobKindData {
        match self.kind {
            JobKind::New => JobKindData::New,
            JobKind::Update => JobKindData::Update {
                llms_txt: self.llms_txt.clone().unwrap_or_default(),
            },
        }
    }

    /// Create database representation from ergonomic JobKindData enum
    pub fn from_kind_data(job_id: Uuid, url: String, status: JobStatus, kind_data: JobKindData) -> Self {
        match kind_data {
            JobKindData::New => JobState {
                job_id,
                url,
                status,
                kind: JobKind::New,
                llms_txt: None,
            },
            JobKindData::Update { llms_txt } => JobState {
                job_id,
                url,
                status,
                kind: JobKind::Update,
                llms_txt: Some(llms_txt),
            },
        }
    }
}

// llms_txt table model (database representation)
#[derive(Debug, Eq, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::llms_txt)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LlmsTxt {
    pub job_id: Uuid,
    pub url: String,
    pub result_data: String,
    pub result_status: ResultStatus,
    pub created_at: DateTime<Utc>,
    pub html_compress: String,
    pub html_checksum: String,
}

impl PartialEq for LlmsTxt {
    // Two LlmsTxt are equivalent if all fields other than created_at are equivalent
    fn eq(&self, other: &LlmsTxt) -> bool {
        self.job_id.eq(&other.job_id) && self.url.eq(&other.url) &&
    self.result_status.eq(&other.result_status) && self.result_data.eq(&other.result_data) &&
      // DO NOT INCLUDE created_at !!
      self.html_compress.eq(&other.html_compress)
    }
}

// LlmsTxtResult - ergonomic Rust enum for the result
/// Result of fetching an llms.txt file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LlmsTxtResult {
    /// Successfully fetched llms.txt content
    Ok { llms_txt: String },
    /// Failed to fetch with error reason
    Error { failure_reason: String },
}

impl LlmsTxt {
    /// Convert database representation to ergonomic Result enum
    pub fn to_result(&self) -> LlmsTxtResult {
        match self.result_status {
            ResultStatus::Ok => LlmsTxtResult::Ok {
                llms_txt: self.result_data.clone(),
            },
            ResultStatus::Error => LlmsTxtResult::Error {
                failure_reason: self.result_data.clone(),
            },
        }
    }

    /// Create database representation from ergonomic Result enum
    pub fn from_result(job_id: Uuid, url: String, result: LlmsTxtResult, html_compress: String) -> Self {
        let created_at = Utc::now();

        // Compute checksum - if normalization fails, use raw HTML
        let html_checksum = core_ltx::web_html::compute_html_checksum(&html_compress).expect("Unexpected: ");

        match result {
            LlmsTxtResult::Ok { llms_txt } => LlmsTxt {
                job_id,
                url,
                result_data: llms_txt,
                result_status: ResultStatus::Ok,
                created_at,
                html_compress,
                html_checksum,
            },
            LlmsTxtResult::Error { failure_reason } => LlmsTxt {
                job_id,
                url,
                result_data: failure_reason,
                result_status: ResultStatus::Error,
                created_at,
                html_compress,
                html_checksum,
            },
        }
    }
}

// API Error Types

/// Error for GET /api/llm_txt endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum GetLlmTxtError {
    /// llms.txt has not been generated for this URL yet
    #[serde(rename = "not_generated")]
    NotGenerated,
    /// Failed llms.txt generation
    #[serde(rename = "generation_failure")]
    GenerationFailure(String),
    /// Unknown error occurred
    #[serde(rename = "unknown")]
    Unknown(String),
}

/// Error for POST /api/llm_txt endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum PostLlmTxtError {
    /// llms.txt has already been generated for this URL
    #[serde(rename = "already_generated")]
    AlreadyGenerated,
    /// llms.txt jobs are in progress for this URL
    #[serde(rename = "jobs_in_progress")]
    JobsInProgress(Vec<Uuid>),
    /// Unknown error occurred
    #[serde(rename = "unknown")]
    Unknown(String),
}

/// Error for PUT /api/llm_txt endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum PutLlmTxtError {
    /// Unknown error occurred
    #[serde(rename = "unknown")]
    Unknown(String),
}

/// Error for GET /api/status endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum StatusError {
    /// The provided job_id is not a valid UUID
    #[serde(rename = "invalid_id")]
    InvalidId,
    /// The job_id was not found in the database
    #[serde(rename = "unknown_id")]
    UnknownId,
    /// Unknown error occurred
    #[serde(rename = "unknown")]
    Unknown(String),
}

/// Error for POST /api/update endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details")]
pub enum UpdateLlmTxtError {
    /// llms.txt has not been generated for this URL yet
    #[serde(rename = "not_generated")]
    NotGenerated,
    /// Unknown error occurred
    #[serde(rename = "unknown")]
    Unknown(String),
}

// API Payload Types

/// Input payload for endpoints that accept a URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlPayload {
    pub url: String,
}

/// Input payload for /api/status endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobIdPayload {
    pub job_id: Uuid,
}

/// Response payload containing a job ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobIdResponse {
    pub job_id: Uuid,
}

/// Response payload for GET /api/llm_txt endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTxtResponse {
    pub content: String,
}

/// Response payload for GET /api/status endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub status: JobStatus,
    pub kind: JobKind,
}

/// Individual item in the list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsTxtListItem {
    pub url: String,
    pub llm_txt: String,
}

/// Response payload for GET /api/list endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsTxtListResponse {
    pub items: Vec<LlmsTxtListItem>,
}

/// Response payload for GET /api/job endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDetailsResponse {
    pub job_id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub kind: JobKind,
    pub llms_txt: Option<String>,
    pub error_message: Option<String>,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

macro_rules! from_error {
    ($lib_err:path, $err_type:tt) => {
        /// Converts a `$lib_err` into an `$err_type::Unknown`.
        impl From<$lib_err> for $err_type {
            fn from(e: $lib_err) -> Self {
                $err_type::Unknown(format!("{:?}", e))
            }
        }
    };
}

macro_rules! from_diesel_not_found_error {
    ($err_type:tt) => {
        /// Converts a `diesel::result::Error::NotFound` into an `$err_type::NotGenerated`
        /// otherwise it's a `$err_type::Unknown(diesel::result::Error)`.
        impl From<diesel::result::Error> for $err_type {
            fn from(e: diesel::result::Error) -> Self {
                match e {
                    diesel::result::Error::NotFound => $err_type::NotGenerated,
                    _ => $err_type::Unknown(format!("{:?}", e)),
                }
            }
        }
    };
}

// GetLlmTxtError

impl IntoResponse for GetLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            GetLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            GetLlmTxtError::Unknown(_) | GetLlmTxtError::GenerationFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(PoolError, GetLlmTxtError);
from_diesel_not_found_error!(GetLlmTxtError);

// PostLlmTxtError

impl IntoResponse for PostLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            PostLlmTxtError::AlreadyGenerated | PostLlmTxtError::JobsInProgress(_) => StatusCode::CONFLICT,
            PostLlmTxtError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(PoolError, PostLlmTxtError);
from_error!(diesel::result::Error, PostLlmTxtError);

// PutLlmTxtError

impl IntoResponse for PutLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(self)).into_response()
    }
}

from_error!(PoolError, PutLlmTxtError);
from_error!(diesel::result::Error, PutLlmTxtError);

// UpdateLlmTxtError

impl IntoResponse for UpdateLlmTxtError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            UpdateLlmTxtError::NotGenerated => StatusCode::NOT_FOUND,
            UpdateLlmTxtError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(PoolError, UpdateLlmTxtError);
from_diesel_not_found_error!(UpdateLlmTxtError);

// StatusError

impl IntoResponse for StatusError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            StatusError::InvalidId => StatusCode::BAD_REQUEST,
            StatusError::UnknownId => StatusCode::NOT_FOUND,
            StatusError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

from_error!(PoolError, StatusError);

impl From<diesel::result::Error> for StatusError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => StatusError::UnknownId,
            _ => StatusError::Unknown(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use core_ltx::web_html::compute_html_checksum;

    use super::*;

    #[test]
    fn test_create_job_state() {
        let job_state = JobState {
            job_id: Uuid::new_v4(),
            url: "https://example.com".to_string(),
            status: JobStatus::Queued,
            kind: JobKind::New,
            llms_txt: None,
        };

        assert!(!job_state.url.is_empty());
        assert_eq!(job_state.status, JobStatus::Queued);
        assert_eq!(job_state.kind, JobKind::New);
        assert_eq!(job_state.llms_txt, None);
    }

    #[test]
    fn test_job_kind_data_conversion() {
        let job_id = Uuid::new_v4();
        let url = "https://example.com".to_string();
        let status = JobStatus::Queued;

        // Test New variant
        let new_kind = JobKindData::New;
        let db_model = JobState::from_kind_data(job_id, url.clone(), status, new_kind.clone());
        assert_eq!(db_model.kind, JobKind::New);
        assert_eq!(db_model.llms_txt, None);
        assert_eq!(db_model.to_kind_data(), new_kind);

        // Test Update variant
        let update_kind = JobKindData::Update {
            llms_txt: "previous content".to_string(),
        };
        let db_model = JobState::from_kind_data(job_id, url.clone(), status, update_kind.clone());
        assert_eq!(db_model.kind, JobKind::Update);
        assert_eq!(db_model.llms_txt, Some("previous content".to_string()));
        assert_eq!(db_model.to_kind_data(), update_kind);
    }

    #[test]
    fn test_create_llms_txt() {
        let html_compress = "<html><body>Test</body></html>".to_string();
        let html_checksum = compute_html_checksum(&html_compress).unwrap();

        let llms_txt = LlmsTxt {
            job_id: Uuid::new_v4(),
            url: "https://example.com/llms.txt".to_string(),
            result_data: "# Example LLMs.txt content".to_string(),
            result_status: ResultStatus::Ok,
            created_at: Utc::now(),
            html_compress: html_compress.clone(),
            html_checksum: html_checksum.clone(),
        };

        assert!(!llms_txt.url.is_empty());
        assert!(!llms_txt.result_data.is_empty());
        assert!(llms_txt.result_data.starts_with("# Example"));
        assert_eq!(llms_txt.result_status, ResultStatus::Ok);
        assert!(!llms_txt.html_compress.is_empty());
        assert!(!llms_txt.html_checksum.is_empty());
        assert_eq!(llms_txt.html_checksum.len(), 32); // MD5 hex is always 32 chars
    }

    #[test]
    fn test_llms_txt_result_conversion() {
        let job_id = Uuid::new_v4();
        let url = "https://example.com/llms.txt".to_string();
        let html_compress = "<html><body>Test</body></html>".to_string();

        // Test Ok variant
        let ok_result = LlmsTxtResult::Ok {
            llms_txt: "content".to_string(),
        };
        let db_model = LlmsTxt::from_result(job_id, url.clone(), ok_result.clone(), html_compress.clone());
        assert_eq!(db_model.result_status, ResultStatus::Ok);
        assert_eq!(db_model.result_data, "content");
        assert_eq!(db_model.html_compress, html_compress);
        assert_eq!(db_model.to_result(), ok_result);

        // Test Error variant
        let error_result = LlmsTxtResult::Error {
            failure_reason: "network timeout".to_string(),
        };
        let db_model = LlmsTxt::from_result(job_id, url.clone(), error_result.clone(), html_compress.clone());
        assert_eq!(db_model.result_status, ResultStatus::Error);
        assert_eq!(db_model.result_data, "network timeout");
        assert_eq!(db_model.html_compress, html_compress);
        assert_eq!(db_model.to_result(), error_result);
    }
}
