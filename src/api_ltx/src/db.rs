use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager, Pool, PoolError, PooledConnection};

pub type Conn = PooledConnection<ConnectionManager<PgConnection>>;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool(database_url: &str) -> Result<DbPool, PoolError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder().build(manager)
}
