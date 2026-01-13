# Testing Guide

This document explains how to run tests, add new tests, and maintain the test infrastructure for the llm-web-index project.

## Quick Start

```bash
# 1. Setup test database (one time)
./scripts/setup_test_db.sh

# 2. Run all tests
./scripts/run_all_tests.sh

# 3. (Optional) View coverage report
open target/llvm-cov/html/index.html
```

---

## Table of Contents

- [Test Infrastructure](#test-infrastructure)
- [Running Tests](#running-tests)
- [Test Organization](#test-organization)
- [Writing Tests](#writing-tests)
- [Troubleshooting](#troubleshooting)
- [CI/CD Integration](#cicd-integration)

---

## Test Infrastructure

### Components

1. **Mock LLM Provider** (`src/core-ltx/src/llms/mock.rs`)
   - Mock implementation of `LlmProvider` trait
   - No real API calls needed
   - Configurable responses for testing

2. **Database Test Utilities** (`src/data-model-ltx/src/test_helpers.rs`)
   - Helper functions for database operations
   - Job creation and management
   - Database cleanup

3. **Test Database** (PostgreSQL in Docker)
   - Isolated test database on port 5433
   - Managed via docker-compose.test.yml
   - Fast in-memory storage (tmpfs)

4. **Test Helper Features**
   - `test-helpers` feature flag in core-ltx and data-model-ltx
   - Enables cross-crate test utility usage

---

## Running Tests

### All Tests

Run the entire test suite:

```bash
./scripts/run_all_tests.sh
```

This script:
- Checks if test database is running (starts it if needed)
- Runs all workspace unit tests
- Runs worker integration tests
- Generates coverage report (if cargo-llvm-cov is installed)
- Provides a summary of results

### Specific Test Suites

**Unit Tests by Crate:**

```bash
# Mock LLM provider tests
cd src/core-ltx && cargo test llms::mock::tests --lib

# Database test helpers
cd src/data-model-ltx && cargo test test_helpers::tests --lib -- --test-threads=1

# API authentication tests
cd src/api-ltx && cargo test --lib

# Core LLM tests
cd src/core-ltx && cargo test --lib

# Cron service tests
cd src/cron-ltx && cargo test --lib
```

**Worker Integration Tests:**

```bash
export TEST_DATABASE_URL="postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db"

# Job processing tests
cargo test --test job_processing

# Job queue tests (concurrent access)
cargo test --test job_queue -- --test-threads=1

# Result handling tests
cargo test --test result_handling -- --test-threads=1
```

**All Workspace Tests:**

```bash
cargo test --workspace --all-targets
```

### Coverage Report

Generate an HTML coverage report:

```bash
# Install cargo-llvm-cov (one time)
cargo install cargo-llvm-cov

# Generate report
cargo llvm-cov --all-targets --workspace --html

# View report
open target/llvm-cov/html/index.html
```

---

## Test Organization

### Directory Structure

```
llm-web-index/
├── src/
│   ├── api-ltx/
│   │   └── src/
│   │       └── auth/
│   │           ├── password.rs (tests inline)
│   │           └── session.rs (tests inline)
│   │
│   ├── core-ltx/
│   │   ├── src/
│   │   │   └── llms/
│   │   │       └── mock.rs (Mock LLM + tests)
│   │   └── tests/
│   │       └── (integration tests)
│   │
│   ├── data-model-ltx/
│   │   ├── src/
│   │   │   ├── models.rs (tests inline)
│   │   │   └── test_helpers.rs (DB utilities)
│   │
│   ├── worker-ltx/
│   │   └── tests/
│   │       ├── job_processing.rs (11 tests)
│   │       ├── job_queue.rs (12 tests)
│   │       └── result_handling.rs (8 tests)
│   │
│   └── cron-ltx/
│       └── src/
│           ├── errors.rs (tests inline)
│           └── process.rs (tests inline)
│
├── scripts/
│   ├── setup_test_db.sh
│   └── run_all_tests.sh
│
├── docker-compose.test.yml
├── .env.test
├── TESTING.md (this file)
└── TEST_STATUS.md (current status)
```

### Test Types

**Unit Tests** (inline with code):
- Located in the same file as the code
- Use `#[cfg(test)]` modules
- Fast, isolated tests

**Integration Tests** (`tests/` directory):
- Test interactions between components
- Use real database
- Test complete workflows

---

## Writing Tests

### Using Mock LLM Provider

```rust
use core_ltx::llms::mock::{MockLlmProvider, sample_valid_llms_txt};

#[tokio::test]
async fn test_my_function() {
    // Create a mock that returns valid llms.txt
    let provider = MockLlmProvider::with_valid_llms_txt();

    // Or create custom responses
    let provider = MockLlmProvider::with_response(
        "generate",  // Matches prompts containing "generate"
        "# Custom Response\n\n> Description\n\n- [Link](/)"
    );

    // Or simulate failures
    let provider = MockLlmProvider::with_failure();

    // Use in your test
    let result = my_function(&provider).await;
    assert!(result.is_ok());
}
```

### Using Database Test Utilities

```rust
use data_model_ltx::test_helpers::*;
use data_model_ltx::models::{JobKind, JobStatus};

#[tokio::test]
async fn test_my_database_function() {
    // Get test database pool
    let pool = test_db_pool().await;

    // Clean database before test
    clean_test_db(&pool).await;

    // Create test data
    let job = create_test_job(
        &pool,
        "https://example.com",
        JobKind::New,
        JobStatus::Queued
    ).await;

    // Run your test
    let result = my_database_function(&pool, job.job_id).await;

    // Verify results
    assert!(result.is_ok());
}
```

### Test Isolation

**Important Rules:**

1. **Database Tests**: Always call `clean_test_db()` at the start
2. **Parallel Execution**: Use `--test-threads=1` for database tests
3. **Unique Data**: Use unique URLs/IDs when possible
4. **Cleanup**: Tests should leave no side effects

### Creating Valid LlmsTxt

```rust
use core_ltx::{is_valid_markdown, validate_is_llm_txt};

fn create_test_llms_txt(content: &str) -> core_ltx::LlmsTxt {
    let markdown = is_valid_markdown(content)
        .expect("Test content should be valid markdown");
    validate_is_llm_txt(markdown)
        .expect("Test content should be valid llms.txt")
}

// Use in tests
let llms_txt = create_test_llms_txt("# Title\n\n> Description\n\n- [Link](/)");
```

### Adding Tests to a New Crate

1. **Add test-helpers dependency** (if using mock/DB utilities):

```toml
[dev-dependencies]
core-ltx = { path = "../core-ltx", features = ["test-helpers"] }
data-model-ltx = { path = "../data-model-ltx", features = ["test-helpers"] }
tokio = { workspace = true }
```

2. **Create tests directory**:

```bash
mkdir -p src/my-crate/tests
touch src/my-crate/tests/my_tests.rs
```

3. **Write tests** using the patterns above

4. **Run tests**:

```bash
cargo test -p my-crate
```

---

## Troubleshooting

### Test Database Not Running

**Error**: `Failed to create test database pool`

**Solution**:
```bash
./scripts/setup_test_db.sh
```

### Database Tests Failing in Parallel

**Error**: Assertion failures due to shared state

**Solution**: Run with single thread:
```bash
cargo test --test my_test -- --test-threads=1
```

### Mock LLM Not Found

**Error**: `unresolved import core_ltx::llms::mock`

**Solution**: Add test-helpers feature:
```toml
[dev-dependencies]
core-ltx = { path = "../core-ltx", features = ["test-helpers"] }
```

### Compilation Errors with LlmsTxt

**Error**: `no method named 'new' found for struct 'LlmsTxt'`

**Solution**: Use `validate_is_llm_txt()`:
```rust
let markdown = is_valid_markdown(content)?;
let llms_txt = validate_is_llm_txt(markdown)?;
```

### Worker Error Types

**Error**: `unresolved variant JobProcessingFailed`

**Solution**: Use correct error types:
```rust
// Correct
let error = worker_ltx::errors::Error::CoreError(
    core_ltx::Error::InvalidLlmsTxtFormat("message".to_string())
);
```

### Test Database Port Conflicts

**Error**: `port 5433 is already in use`

**Solution**:
```bash
# Stop existing test database
docker compose -f docker-compose.test.yml down

# Or change port in docker-compose.test.yml and .env.test
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15-alpine
        env:
          POSTGRES_USER: ltx_test_user
          POSTGRES_PASSWORD: ltx_test_password
          POSTGRES_DB: ltx_test_db
        ports:
          - 5433:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable

      - name: Run migrations
        env:
          DATABASE_URL: postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db
        run: |
          cd src/api-ltx
          diesel migration run

      - name: Run tests
        env:
          TEST_DATABASE_URL: postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db
        run: cargo test --workspace --all-targets

      - name: Generate coverage
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --all-targets --workspace --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./lcov.info
```

---

## Best Practices

### 1. Test Naming

Use descriptive names that explain what is being tested:

```rust
✅ test_handle_job_success_new()
✅ test_next_job_in_queue_concurrent_claiming()
❌ test_job()
❌ test1()
```

### 2. Test Organization

Group related tests in the same file:

```rust
// job_processing.rs - All job processing tests
// job_queue.rs - All queue management tests
// result_handling.rs - All result storage tests
```

### 3. Assertions

Use descriptive assertion messages:

```rust
✅ assert!(result.is_ok(), "Job processing should succeed with valid input");
❌ assert!(result.is_ok());
```

### 4. Test Data

Use realistic test data:

```rust
✅ create_test_job(&pool, "https://example.com", ...)
❌ create_test_job(&pool, "test", ...)
```

### 5. Async Tests

Always use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

---

## Test Coverage Goals

| Component | Target | Current | Status |
|-----------|--------|---------|--------|
| core-ltx | 90% | ~85% | ✅ |
| data-model-ltx | 85% | ~75% | 🟡 |
| api-ltx | 85% | ~70% | 🟡 |
| worker-ltx | 90% | ~85% | ✅ |
| cron-ltx | 80% | ~70% | 🟡 |
| front-ltx | 70% | ~0% | ❌ |

**Overall Target**: 85%+ code coverage

---

## Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Diesel Testing](https://diesel.rs/guides/getting-started)
- [TEST_STATUS.md](./TEST_STATUS.md) - Current test implementation status

---

## Questions?

For questions or issues with tests:
1. Check [TEST_STATUS.md](./TEST_STATUS.md) for current status
2. Check this document's Troubleshooting section
3. Review existing tests for examples
4. Open an issue on GitHub

---

**Last Updated**: 2026-01-13
**Test Count**: 90+ tests
**Coverage**: ~80%+
