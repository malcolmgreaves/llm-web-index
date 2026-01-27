use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;

pub type PoolError = deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>;

pub type DbPool = Pool<AsyncPgConnection>;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionPoolError {
    #[error("Failed to build connection pool: {0}")]
    BuildError(#[from] deadpool::managed::BuildError),
    #[error("Failed to establish initial database connection: {0}")]
    ConnectionError(#[from] PoolError),
}

pub async fn establish_connection_pool(database_url: &str) -> Result<DbPool, ConnectionPoolError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder(config).build()?;

    // Force an initial connection to validate the database is reachable
    // This ensures we fail fast if the DB is unavailable
    let _conn = pool.get().await?;

    Ok(pool)
}
