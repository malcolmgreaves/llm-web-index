use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// job_state table model
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::job_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct JobState {
    pub job_id: Uuid,
    pub url: String,
    pub status: String,
    pub kind: String,
}

// llms_txt table model
#[derive(Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::llms_txt)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LlmsTxt {
    pub job_id: Uuid,
    pub url: String,
    pub result: String,
}

// names table models
#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::names)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Name {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = crate::schema::names)]
pub struct NewName {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_state() {
        let job_state = JobState {
            job_id: Uuid::new_v4(),
            url: "https://example.com".to_string(),
            status: "pending".to_string(),
            kind: "scrape".to_string(),
        };

        assert!(!job_state.url.is_empty());
        assert_eq!(job_state.status, "pending");
        assert_eq!(job_state.kind, "scrape");
    }

    #[test]
    fn test_create_llms_txt() {
        let llms_txt = LlmsTxt {
            job_id: Uuid::new_v4(),
            url: "https://example.com/llms.txt".to_string(),
            result: "# Example LLMs.txt content".to_string(),
        };

        assert!(!llms_txt.url.is_empty());
        assert!(!llms_txt.result.is_empty());
        assert!(llms_txt.result.starts_with("# Example"));
    }
}
