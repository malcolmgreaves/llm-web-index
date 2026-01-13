pub mod auth_client;
pub mod errors;
pub mod process;

pub use auth_client::AuthenticatedClient;
pub use errors::Error;
pub use process::poll_and_process;

use data_model_ltx::models::{JobKind, ResultStatus};
use diesel::prelude::*;

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
