//! Tests for job processing logic
//!
//! This module tests the handle_job() function which is responsible for:
//! - Downloading HTML from URLs
//! - Generating or updating llms.txt using LLM providers
//! - Handling various error conditions

use core_ltx::llms::mock::{MockLlmProvider, sample_valid_llms_txt};
use data_model_ltx::models::{JobKindData, JobState, JobStatus};
use worker_ltx::work::{JobResult, handle_job};

/// Helper to create a test job without database
fn create_test_job_for_processing(url: &str, kind_data: JobKindData) -> JobState {
    JobState::from_kind_data(uuid::Uuid::new_v4(), url.to_string(), JobStatus::Queued, kind_data)
}

#[tokio::test]
async fn test_handle_job_success_new() {
    let provider = MockLlmProvider::with_valid_llms_txt();

    // Note: Using a real URL that should be accessible
    // In a real test environment, you might want to use a local test server
    let job = create_test_job_for_processing("https://example.com", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::Success { html, llms_txt } => {
            assert!(!html.is_empty(), "HTML should not be empty");
            assert!(
                llms_txt.md_content().contains("# Example"),
                "llms.txt should contain expected content"
            );
        }
        JobResult::GenerationFailed { html, error } => {
            panic!(
                "Expected success but got generation failure. HTML: {}, Error: {}",
                html, error
            );
        }
        JobResult::DownloadFailed { error } => {
            panic!("Expected success but got download failure: {}", error);
        }
    }
}

#[tokio::test]
async fn test_handle_job_success_update() {
    let provider = MockLlmProvider::with_valid_llms_txt();

    let job = create_test_job_for_processing(
        "https://example.com",
        JobKindData::Update {
            llms_txt: "# Old Content\n\n> Old description\n\n- [Link](/)".to_string(),
        },
    );

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::Success { html, llms_txt } => {
            assert!(!html.is_empty());
            // The mock should return updated content
            assert!(llms_txt.md_content().contains("#"));
        }
        _ => panic!("Expected successful update"),
    }
}

#[tokio::test]
async fn test_handle_job_generation_failed() {
    // Mock provider that always fails
    let provider = MockLlmProvider::with_failure();

    let job = create_test_job_for_processing("https://example.com", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::GenerationFailed { html, error } => {
            assert!(!html.is_empty(), "HTML should be preserved even on LLM failure");
            assert!(
                error.to_string().contains("Mock LLM"),
                "Error should mention mock provider failure"
            );
        }
        JobResult::Success { .. } => {
            panic!("Expected generation failure but got success");
        }
        JobResult::DownloadFailed { .. } => {
            panic!("Expected generation failure but got download failure");
        }
    }
}

#[tokio::test]
async fn test_handle_job_download_failed_invalid_url() {
    let provider = MockLlmProvider::with_valid_llms_txt();

    // Invalid URL that should fail
    let job = create_test_job_for_processing("not-a-valid-url", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::DownloadFailed { error } => {
            // Expected - no HTML to preserve
            assert!(!error.to_string().is_empty());
        }
        _ => panic!("Expected download failure for invalid URL"),
    }
}

#[tokio::test]
async fn test_handle_job_download_failed_unreachable_host() {
    let provider = MockLlmProvider::with_valid_llms_txt();

    // Valid URL format but unreachable host
    let job = create_test_job_for_processing(
        "https://this-domain-definitely-does-not-exist-12345.com",
        JobKindData::New,
    );

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::DownloadFailed { error } => {
            // Expected - network error
            assert!(!error.to_string().is_empty());
        }
        _ => panic!("Expected download failure for unreachable host"),
    }
}

#[tokio::test]
async fn test_handle_job_invalid_markdown_from_llm() {
    // Mock that returns invalid markdown
    let provider = MockLlmProvider::with_invalid_markdown();

    let job = create_test_job_for_processing("https://example.com", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::GenerationFailed { html, error } => {
            assert!(!html.is_empty(), "HTML should be preserved");
            // The error should indicate markdown validation failure
            assert!(!error.to_string().is_empty());
        }
        _ => panic!("Expected generation failure for invalid markdown"),
    }
}

#[tokio::test]
async fn test_handle_job_invalid_llms_txt_format() {
    // Mock that returns valid markdown but invalid llms.txt format
    let provider = MockLlmProvider::with_invalid_llms_txt();

    let job = create_test_job_for_processing("https://example.com", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::GenerationFailed { html, error } => {
            assert!(!html.is_empty(), "HTML should be preserved");
            // Error should indicate llms.txt format validation failure
            assert!(!error.to_string().is_empty());
        }
        _ => panic!("Expected generation failure for invalid llms.txt format"),
    }
}

#[tokio::test]
async fn test_handle_job_preserves_html_on_llm_failure() {
    let provider = MockLlmProvider::with_failure();

    let job = create_test_job_for_processing("https://example.com", JobKindData::New);

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::GenerationFailed { html, error: _ } => {
            // Verify HTML was actually downloaded
            assert!(html.len() > 100, "HTML should contain actual content");
            assert!(
                html.contains("<html") || html.contains("<!DOCTYPE"),
                "HTML should look like real HTML"
            );
        }
        _ => panic!("Expected generation failure"),
    }
}

#[tokio::test]
async fn test_handle_job_update_with_existing_content() {
    let provider = MockLlmProvider::with_response(
        "update",
        r#"# Updated Content

> Updated description

- [Home](/)
- [New Link](/new)

## Updates
- [Changelog](/changelog)
"#,
    );

    let existing_llms_txt = r#"# Old Content

> Old description

- [Home](/)
"#;

    let job = create_test_job_for_processing(
        "https://example.com",
        JobKindData::Update {
            llms_txt: existing_llms_txt.to_string(),
        },
    );

    let result = handle_job(&provider, &job).await;

    match result {
        JobResult::Success { html, llms_txt } => {
            assert!(!html.is_empty());
            let content = llms_txt.md_content();
            assert!(
                content.contains("Updated"),
                "Content should be updated, not the old content"
            );
        }
        _ => panic!("Expected successful update"),
    }
}

#[tokio::test]
async fn test_handle_job_new_vs_update_distinction() {
    let provider = MockLlmProvider::with_valid_llms_txt();

    // Test New job
    let new_job = create_test_job_for_processing("https://example.com", JobKindData::New);
    let new_result = handle_job(&provider, &new_job).await;
    assert!(
        matches!(new_result, JobResult::Success { .. }),
        "New job should succeed"
    );

    // Test Update job
    let update_job = create_test_job_for_processing(
        "https://example.com",
        JobKindData::Update {
            llms_txt: "# Existing\n\n> Content\n\n- [Link](/)".to_string(),
        },
    );
    let update_result = handle_job(&provider, &update_job).await;
    assert!(
        matches!(update_result, JobResult::Success { .. }),
        "Update job should succeed"
    );
}

#[tokio::test]
async fn test_handle_job_with_multiple_responses() {
    // Mock with different responses based on prompt content
    let provider = MockLlmProvider::with_responses(vec![
        ("generate", sample_valid_llms_txt()),
        ("update", sample_valid_llms_txt()),
    ]);

    let job = create_test_job_for_processing("https://example.com", JobKindData::New);
    let result = handle_job(&provider, &job).await;

    assert!(
        matches!(result, JobResult::Success { .. }),
        "Should use appropriate response based on job kind"
    );
}
