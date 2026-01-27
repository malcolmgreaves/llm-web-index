//! Tests for result handling operations
//!
//! This module tests the handle_result() function which is responsible for:
//! - Inserting llms_txt records for successful jobs
//! - Updating job_state status appropriately
//! - Handling four result types: Success, GenerationFailed, DownloadFailed, HtmlProcessingFailed
//! - Ensuring database transactions are atomic

use core_ltx::{
    compress_string, decompress_to_string, is_valid_markdown, normalize_html, validate_is_llm_txt,
    web_html::compute_html_checksum,
};
use data_model_ltx::{
    models::{JobKind, JobStatus, ResultStatus},
    test_helpers::{TestDbGuard, clean_test_db, create_test_job, get_job_by_id, get_llms_txt_by_job_id, test_db_pool},
};
use diesel::IntoSql;
use tokio::sync::Mutex;
use worker_ltx::work::{JobResult, handle_result};

/// Helper to create a valid LlmsTxt for testing
fn create_test_llms_txt(content: &str) -> core_ltx::LlmsTxt {
    let markdown = is_valid_markdown(content).expect("Test content should be valid markdown");
    validate_is_llm_txt(markdown).expect("Test content should be valid llms.txt")
}

/// Helper to create a test error
fn create_test_error(message: &str) -> worker_ltx::Error {
    worker_ltx::Error::CoreError(core_ltx::Error::InvalidLlmsTxtFormat(message.to_string()))
}

/// Helper to compress HTML and compute checksum for tests
fn compress_html(html: &str) -> (Vec<u8>, String) {
    let normalized_html = normalize_html(html).expect("Failed to parse & clean HTML");
    let html_compress = compress_string(normalized_html.as_str()).expect("Failed to compress HTML");
    let html_checksum = compute_html_checksum(&normalized_html).expect("Failed to compute checksum");
    (html_compress, html_checksum)
}

static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_handle_result_success() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let (html_compress, html_checksum) = compress_html("<html><body><h1>Test</h1></body></html>");
    let llms_txt = create_test_llms_txt("# Test Site\n\n> Test\n\n- [Home](/)");

    let result = JobResult::Success {
        html_compress: html_compress.clone(),
        html_checksum: html_checksum.clone(),
        llms_txt,
    };

    handle_result(&pool, &job, result).await.unwrap();

    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Success);

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await.unwrap();
    assert_eq!(llms_txt_record.job_id, job.job_id);
    assert_eq!(llms_txt_record.url, job.url);
    assert_eq!(llms_txt_record.result_status, ResultStatus::Ok);
    assert_eq!(llms_txt_record.html_compress, html_compress);
    assert_eq!(llms_txt_record.html_checksum, html_checksum);
    assert!(llms_txt_record.result_data.contains("# Test Site"));
}

#[tokio::test]
async fn test_handle_result_generation_failed() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let (html_compress, html_checksum) = compress_html("<html><body><h1>Test</h1></body></html>");
    let error = create_test_error("LLM generation failed");

    let result = JobResult::GenerationFailed {
        html_compress: html_compress.clone(),
        html_checksum: html_checksum.clone(),
        error,
    };

    handle_result(&pool, &job, result).await.unwrap();

    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Failure);

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await.unwrap();
    assert_eq!(llms_txt_record.job_id, job.job_id);
    assert_eq!(llms_txt_record.result_status, ResultStatus::Error);
    assert_eq!(llms_txt_record.html_compress, html_compress, "HTML should be preserved");
    assert!(
        llms_txt_record.result_data.contains("LLM generation failed"),
        "Error message should be stored"
    );
}

#[tokio::test]
async fn test_handle_result_download_failed() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let error = create_test_error("Download failed");

    let result = JobResult::DownloadFailed { error };

    handle_result(&pool, &job, result).await.unwrap();

    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Failure);

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await;
    assert!(
        llms_txt_record.is_none(),
        "Should not create llms_txt record for download failures"
    );
}

#[tokio::test]
async fn test_handle_result_html_processing_failed() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let error = create_test_error("HTML normalization failed");

    let result = JobResult::HtmlProcessingFailed { error };

    handle_result(&pool, &job, result).await.unwrap();

    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    assert_eq!(updated_job.status, JobStatus::Failure);

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await;
    assert!(
        llms_txt_record.is_none(),
        "Should not create llms_txt record for HTML processing failures"
    );
}

#[tokio::test]
async fn test_handle_result_preserves_html_on_generation_failure() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let normalized_html = normalize_html(&format!("<html><body>{}</body></html>", "X".repeat(10000)))
        .expect("Could not parse & clean HTML");
    let (html_compress, html_checksum) = compress_html(normalized_html.as_str());

    let error = create_test_error("Generation error");

    let result = JobResult::GenerationFailed {
        html_compress: html_compress.clone(),
        html_checksum,
        error,
    };

    handle_result(&pool, &job, result).await.unwrap();

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await.unwrap();
    // Verify we can decompress and get original HTML
    let decompressed = decompress_to_string(&llms_txt_record.html_compress).unwrap();
    assert_eq!(decompressed.len(), normalized_html.as_str().len());
}

#[tokio::test]
async fn test_handle_result_transaction_atomicity_success() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let (html_compress, html_checksum) = compress_html("<html></html>");
    let result = JobResult::Success {
        html_compress,
        html_checksum,
        llms_txt: create_test_llms_txt("# Test\n\n> Test\n\n- [Link](/)"),
    };

    handle_result(&pool, &job, result).await.unwrap();

    let updated_job = get_job_by_id(&pool, job.job_id).await.unwrap();
    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await.unwrap();

    assert_eq!(updated_job.status, JobStatus::Success);
    assert_eq!(llms_txt_record.result_status, ResultStatus::Ok);
}

#[tokio::test]
async fn test_handle_result_multiple_jobs() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job1 = create_test_job(&pool, "https://job1.com", JobKind::New, JobStatus::Running).await;
    let job2 = create_test_job(&pool, "https://job2.com", JobKind::New, JobStatus::Running).await;
    let job3 = create_test_job(&pool, "https://job3.com", JobKind::New, JobStatus::Running).await;

    let (html_compress1, html_checksum1) = compress_html("<html>1</html>");
    let (html_compress2, html_checksum2) = compress_html("<html>2</html>");

    handle_result(
        &pool,
        &job1,
        JobResult::Success {
            html_compress: html_compress1,
            html_checksum: html_checksum1,
            llms_txt: create_test_llms_txt("# Job 1\n\n> Test\n\n- [Link](/)"),
        },
    )
    .await
    .unwrap();

    handle_result(
        &pool,
        &job2,
        JobResult::GenerationFailed {
            html_compress: html_compress2,
            html_checksum: html_checksum2,
            error: create_test_error("Error 2"),
        },
    )
    .await
    .unwrap();

    handle_result(
        &pool,
        &job3,
        JobResult::DownloadFailed {
            error: create_test_error("Error 3"),
        },
    )
    .await
    .unwrap();

    let updated_job1 = get_job_by_id(&pool, job1.job_id).await.unwrap();
    let updated_job2 = get_job_by_id(&pool, job2.job_id).await.unwrap();
    let updated_job3 = get_job_by_id(&pool, job3.job_id).await.unwrap();

    assert_eq!(updated_job1.status, JobStatus::Success);
    assert_eq!(updated_job2.status, JobStatus::Failure);
    assert_eq!(updated_job3.status, JobStatus::Failure);

    assert!(get_llms_txt_by_job_id(&pool, job1.job_id).await.is_some());
    assert!(get_llms_txt_by_job_id(&pool, job2.job_id).await.is_some());
    assert!(get_llms_txt_by_job_id(&pool, job3.job_id).await.is_none());
}

#[tokio::test]
async fn test_handle_result_error_message_storage() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job = create_test_job(&pool, "https://example.com", JobKind::New, JobStatus::Running).await;

    let error_message = "Very specific error: Invalid format at line 42";
    let (html_compress, html_checksum) = compress_html("<html></html>");
    let result = JobResult::GenerationFailed {
        html_compress,
        html_checksum,
        error: create_test_error(error_message),
    };

    handle_result(&pool, &job, result).await.unwrap();

    let llms_txt_record = get_llms_txt_by_job_id(&pool, job.job_id).await.unwrap();
    assert!(
        llms_txt_record.result_data.contains(error_message),
        "Error message should be stored"
    );
}

#[tokio::test]
async fn test_handle_result_concurrent_results() {
    let _db = TestDbGuard::acquire().await;
    let pool = test_db_pool().await;
    let _guard = TEST_MUTEX.lock().await;
    clean_test_db(&pool).await;

    let job1 = create_test_job(&pool, "https://job1.com", JobKind::New, JobStatus::Running).await;
    let job2 = create_test_job(&pool, "https://job2.com", JobKind::New, JobStatus::Running).await;
    let job3 = create_test_job(&pool, "https://job3.com", JobKind::New, JobStatus::Running).await;

    // Save IDs for later verification
    let job1_id = job1.job_id;
    let job2_id = job2.job_id;
    let job3_id = job3.job_id;

    let pool1 = pool.clone();
    let pool2 = pool.clone();
    let pool3 = pool.clone();

    let (html_compress1, html_checksum1) = compress_html("<html>1</html>");
    let (html_compress2, html_checksum2) = compress_html("<html>2</html>");
    let (html_compress3, html_checksum3) = compress_html("<html>3</html>");

    let handle1 = tokio::spawn(async move {
        handle_result(
            &pool1,
            &job1,
            JobResult::Success {
                html_compress: html_compress1,
                html_checksum: html_checksum1,
                llms_txt: create_test_llms_txt("# Job 1\n\n> Test\n\n- [Link](/)"),
            },
        )
        .await
    });

    let handle2 = tokio::spawn(async move {
        handle_result(
            &pool2,
            &job2,
            JobResult::Success {
                html_compress: html_compress2,
                html_checksum: html_checksum2,
                llms_txt: create_test_llms_txt("# Job 2\n\n> Test\n\n- [Link](/)"),
            },
        )
        .await
    });

    let handle3 = tokio::spawn(async move {
        handle_result(
            &pool3,
            &job3,
            JobResult::Success {
                html_compress: html_compress3,
                html_checksum: html_checksum3,
                llms_txt: create_test_llms_txt("# Job 3\n\n> Test\n\n- [Link](/)"),
            },
        )
        .await
    });

    assert!(handle1.await.unwrap().is_ok());
    assert!(handle2.await.unwrap().is_ok());
    assert!(handle3.await.unwrap().is_ok());

    assert_eq!(get_job_by_id(&pool, job1_id).await.unwrap().status, JobStatus::Success);
    assert_eq!(get_job_by_id(&pool, job2_id).await.unwrap().status, JobStatus::Success);
    assert_eq!(get_job_by_id(&pool, job3_id).await.unwrap().status, JobStatus::Success);
}
