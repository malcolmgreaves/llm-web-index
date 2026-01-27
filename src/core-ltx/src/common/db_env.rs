use std::env::VarError;

use crate::common::db::{DbPool, establish_connection_pool};

/// Uses the env var DATABASE_URL to establish a database connection pool using diesel.
/// WARNING: Panics if DATABASE_URL is not set or if the connection fails!
pub async fn get_db_pool() -> DbPool {
    let database_url = get_database_url().expect("DATABASE_URL must be set in .env file or present as an env var");

    match establish_connection_pool(&database_url).await {
        Ok(p) => p,
        Err(e) => panic!("Couldn't connect to the database ({}): {}", database_url, e),
    }
}

/// Retrieves the value for the env var DATABASE_URL.
pub fn get_database_url() -> Result<String, VarError> {
    std::env::var("DATABASE_URL")
}
