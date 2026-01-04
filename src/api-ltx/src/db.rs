use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;

pub type DbPool = Pool<AsyncPgConnection>;

pub fn establish_connection_pool(
    database_url: &str,
) -> Result<DbPool, deadpool::managed::BuildError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Pool::builder(config).build()
}
