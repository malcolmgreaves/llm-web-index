#[derive(Debug)]
pub enum Error {
    RecordNotFound,
    DbError(diesel::result::Error),
    DbPoolError(String),
    // ...what else?
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordNotFound => write!(f, "Record not found in database."),
            Self::DbError(diesel_error) => write!(f, "Database error: {}", diesel_error),
            Self::DbPoolError(pool_error_desc) => write!(f, "Database pool error: {}", pool_error_desc),
            // ...
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::RecordNotFound,
            _ => Self::DbError(error),
        }
    }
}

// PoolError
impl<E: std::fmt::Debug> From<deadpool::managed::PoolError<E>> for Error {
    fn from(error: deadpool::managed::PoolError<E>) -> Self {
        Self::DbPoolError(format!("{:?}", error))
    }
}

// ...?
