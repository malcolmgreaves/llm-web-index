use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SqlType;
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

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
    pub fn from_kind_data(
        job_id: Uuid,
        url: String,
        status: JobStatus,
        kind_data: JobKindData,
    ) -> Self {
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
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::llms_txt)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LlmsTxt {
    pub job_id: Uuid,
    pub url: String,
    pub result_data: String,
    pub result_status: ResultStatus,
    pub created_at: DateTime<Utc>,
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
    pub fn from_result(job_id: Uuid, url: String, result: LlmsTxtResult) -> Self {
        let created_at = Utc::now();
        match result {
            LlmsTxtResult::Ok { llms_txt } => LlmsTxt {
                job_id,
                url,
                result_data: llms_txt,
                result_status: ResultStatus::Ok,
                created_at,
            },
            LlmsTxtResult::Error { failure_reason } => LlmsTxt {
                job_id,
                url,
                result_data: failure_reason,
                result_status: ResultStatus::Error,
                created_at,
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

#[cfg(test)]
mod tests {
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
        let llms_txt = LlmsTxt {
            job_id: Uuid::new_v4(),
            url: "https://example.com/llms.txt".to_string(),
            result_data: "# Example LLMs.txt content".to_string(),
            result_status: ResultStatus::Ok,
            created_at: Utc::now(),
        };

        assert!(!llms_txt.url.is_empty());
        assert!(!llms_txt.result_data.is_empty());
        assert!(llms_txt.result_data.starts_with("# Example"));
        assert_eq!(llms_txt.result_status, ResultStatus::Ok);
    }

    #[test]
    fn test_llms_txt_result_conversion() {
        let job_id = Uuid::new_v4();
        let url = "https://example.com/llms.txt".to_string();

        // Test Ok variant
        let ok_result = LlmsTxtResult::Ok {
            llms_txt: "content".to_string(),
        };
        let db_model = LlmsTxt::from_result(job_id, url.clone(), ok_result.clone());
        assert_eq!(db_model.result_status, ResultStatus::Ok);
        assert_eq!(db_model.result_data, "content");
        assert_eq!(db_model.to_result(), ok_result);

        // Test Error variant
        let error_result = LlmsTxtResult::Error {
            failure_reason: "network timeout".to_string(),
        };
        let db_model = LlmsTxt::from_result(job_id, url.clone(), error_result.clone());
        assert_eq!(db_model.result_status, ResultStatus::Error);
        assert_eq!(db_model.result_data, "network timeout");
        assert_eq!(db_model.to_result(), error_result);
    }
}
