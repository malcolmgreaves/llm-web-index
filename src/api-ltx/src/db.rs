use sqlx::Error as SqlxError;
use sqlx::postgres::{PgPool, PgPoolOptions};

pub type DbPool = PgPool;

pub async fn establish_connection_pool(database_url: &str) -> Result<DbPool, SqlxError> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
