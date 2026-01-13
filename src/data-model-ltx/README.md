# data-model-ltx

The application's data model layer. Defines database schemas, models, and database operations for managing llms.txt generation jobs and results.

## Overview

The `data-model-ltx` crate provides:

- **Database schema**: Diesel schema definitions for all tables
- **Data models**: Rust structs representing database records
- **CRUD operations**: Functions for creating, reading, updating, and deleting records
- **Database utilities**: Connection management, transactions, and helpers
- **Type safety**: Compile-time guarantees for database operations via Diesel ORM

## Architecture

```
src/data-model-ltx/
├── src/
│   ├── lib.rs      # Module exports
│   ├── schema.rs   # Diesel schema definitions (generated from migrations)
│   ├── models.rs   # Rust structs for database records
│   └── db.rs       # Database operations and utilities
└── Cargo.toml
```

## Database Schema

The application uses PostgreSQL with the following main tables:

### jobs

Tracks llms.txt generation and update jobs:

- `id` (UUID): Primary key, unique job identifier
- `url` (TEXT): Website URL to generate llms.txt for
- `status` (TEXT): Job status (pending, in_progress, completed, failed)
- `result` (TEXT, nullable): Generated llms.txt content (when completed)
- `error_message` (TEXT, nullable): Error details (when failed)
- `created_at` (TIMESTAMP): When the job was created
- `updated_at` (TIMESTAMP): Last status update time
- `started_at` (TIMESTAMP, nullable): When processing began
- `completed_at` (TIMESTAMP, nullable): When processing finished

### websites

Stores information about websites being monitored:

- `id` (UUID): Primary key
- `url` (TEXT): Website URL (unique)
- `last_generated_at` (TIMESTAMP, nullable): Last successful generation
- `update_interval_hours` (INTEGER): How often to update (default: 24)
- `last_llms_txt` (TEXT, nullable): Most recent generated content
- `created_at` (TIMESTAMP): When website was first added
- `updated_at` (TIMESTAMP): Last modification time

## Data Models

### Job

Represents a single generation or update job:

```rust
pub struct Job {
    pub id: Uuid,
    pub url: String,
    pub status: JobStatus,
    pub result: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub started_at: Option<chrono::NaiveDateTime>,
    pub completed_at: Option<chrono::NaiveDateTime>,
}
```

### JobStatus

Enum representing job states:

```rust
pub enum JobStatus {
    Pending,      // Waiting to be processed
    InProgress,   // Currently being processed by worker
    Completed,    // Successfully generated
    Failed,       // Failed with error
}
```

### NewJob

For creating new jobs:

```rust
pub struct NewJob {
    pub url: String,
}
```

### Website

Represents a website being monitored:

```rust
pub struct Website {
    pub id: Uuid,
    pub url: String,
    pub last_generated_at: Option<chrono::NaiveDateTime>,
    pub update_interval_hours: i32,
    pub last_llms_txt: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
```

## Database Operations

The `db` module provides functions for common operations:

### Job Operations

```rust
// Create a new job
pub async fn create_job(pool: &DbPool, url: &str) -> Result<Job, DbError>;

// Get job by ID
pub async fn get_job(pool: &DbPool, id: Uuid) -> Result<Option<Job>, DbError>;

// Update job status
pub async fn update_job_status(
    pool: &DbPool,
    id: Uuid,
    status: JobStatus
) -> Result<(), DbError>;

// Mark job as completed with result
pub async fn complete_job(
    pool: &DbPool,
    id: Uuid,
    result: &str
) -> Result<(), DbError>;

// Mark job as failed with error
pub async fn fail_job(
    pool: &DbPool,
    id: Uuid,
    error: &str
) -> Result<(), DbError>;

// Get all pending jobs
pub async fn get_pending_jobs(pool: &DbPool) -> Result<Vec<Job>, DbError>;
```

### Website Operations

```rust
// Add or update a website
pub async fn upsert_website(
    pool: &DbPool,
    url: &str,
    interval_hours: i32
) -> Result<Website, DbError>;

// Get website by URL
pub async fn get_website(
    pool: &DbPool,
    url: &str
) -> Result<Option<Website>, DbError>;

// Get websites needing updates
pub async fn get_websites_needing_update(
    pool: &DbPool
) -> Result<Vec<Website>, DbError>;

// Update website's last generation time and content
pub async fn update_website_generation(
    pool: &DbPool,
    url: &str,
    content: &str
) -> Result<(), DbError>;
```

## Usage Examples

### Creating a Job

```rust
use data_model_ltx::db::{create_job, get_db_pool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    let job = create_job(&pool, "https://example.com").await?;
    println!("Created job: {}", job.id);

    Ok(())
}
```

### Processing Jobs

```rust
use data_model_ltx::db::{get_pending_jobs, update_job_status, complete_job};
use data_model_ltx::models::JobStatus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    // Get pending jobs
    let jobs = get_pending_jobs(&pool).await?;

    for job in jobs {
        // Mark as in progress
        update_job_status(&pool, job.id, JobStatus::InProgress).await?;

        // Process job (generate llms.txt)
        let result = generate_llms_txt(&job.url).await?;

        // Mark as completed
        complete_job(&pool, job.id, &result).await?;
    }

    Ok(())
}
```

### Checking for Updates

```rust
use data_model_ltx::db::{get_websites_needing_update, create_job};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    // Get websites that haven't been updated recently
    let websites = get_websites_needing_update(&pool).await?;

    // Create update jobs for them
    for website in websites {
        create_job(&pool, &website.url).await?;
        println!("Created update job for {}", website.url);
    }

    Ok(())
}
```

## Connection Management

The crate uses deadpool for connection pooling:

```rust
use data_model_ltx::db::get_db_pool;

// Get a connection pool (reads DATABASE_URL from environment)
let pool = get_db_pool().await?;

// Pool automatically manages connections
// Connections are returned to pool after use
```

Connection pool configuration:
- Maximum connections: 10 (configurable)
- Automatic reconnection on connection loss
- Connection timeout: 30 seconds
- Async-friendly with tokio integration

## Migrations

Database schema is managed with Diesel migrations located in `../api-ltx/migrations/`:

```bash
# Create a new migration
diesel migration generate add_websites_table

# Apply pending migrations
diesel migration run

# Revert last migration
diesel migration revert

# Regenerate schema.rs
diesel print-schema > src/data-model-ltx/src/schema.rs
```

Migrations run automatically in Docker on container startup.

## Error Handling

All database operations return `Result` types:

```rust
pub enum DbError {
    ConnectionError(String),
    QueryError(String),
    NotFound,
    DuplicateKey,
    InvalidData(String),
}
```

Errors include context for debugging and proper error propagation through the application.

## Testing

```bash
# Run unit tests (requires test database)
cargo test -p data-model-ltx

# Run with coverage
just test
```

### Test Database Setup

```bash
# Create test database
createdb ltx_db_test

# Run migrations on test database
DATABASE_URL='postgres://ltx_user:ltx_password@localhost/ltx_db_test' \
  diesel migration run

# Run tests
DATABASE_URL='postgres://ltx_user:ltx_password@localhost/ltx_db_test' \
  cargo test -p data-model-ltx
```

## Dependencies

Key dependencies:

- `diesel`: ORM and query builder
- `diesel-async`: Async database operations
- `deadpool`: Connection pooling
- `uuid`: UUID generation and handling
- `chrono`: Date and time handling
- `serde`: Serialization for API responses
- `anyhow`: Error handling
- `thiserror`: Custom error types

See [Cargo.toml](Cargo.toml) for the complete dependency list.

## Performance Considerations

- **Connection pooling**: Reuses database connections efficiently
- **Prepared statements**: Diesel uses prepared statements for performance and security
- **Indexes**: Database indexes on frequently queried columns (id, url, status, created_at)
- **Async operations**: Non-blocking database access with tokio
- **Batch operations**: Use transactions for multiple related operations

## Development Guidelines

### Adding New Tables

1. Create a new migration: `diesel migration generate add_new_table`
2. Write SQL in the generated `up.sql` and `down.sql` files
3. Run the migration: `diesel migration run`
4. Regenerate schema: `diesel print-schema > src/data-model-ltx/src/schema.rs`
5. Add corresponding Rust structs in `models.rs`
6. Add CRUD operations in `db.rs`

### Modifying Existing Tables

1. Create a migration for the change
2. Test both `up` and `down` migrations
3. Update corresponding Rust structs
4. Update any affected database operations
5. Update tests to reflect schema changes

## Related Documentation

- [Diesel Documentation](https://diesel.rs/) - ORM reference
- [PostgreSQL Documentation](https://www.postgresql.org/docs/) - Database reference
- [Project Root README](../../README.md) - Overall project documentation
- [api-ltx README](../api-ltx/README.md) - API server that uses this data model
- [worker-ltx README](../worker-ltx/README.md) - Worker that uses this data model
- [cron-ltx README](../cron-ltx/README.md) - Cron service that uses this data model
