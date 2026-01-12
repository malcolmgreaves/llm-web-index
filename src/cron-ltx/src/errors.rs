#[derive(Debug)]
pub enum Error {
    RecordNotFound,
    DbError(diesel::result::Error),
    DbPoolError(String),
    InvalidUrl(url::ParseError),
    HttpError(reqwest::Error),
    CoreError(core_ltx::Error),
    JobInProgress,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordNotFound => write!(f, "Record not found in database"),
            Self::DbError(e) => write!(f, "Database error: {}", e),
            Self::DbPoolError(s) => write!(f, "Database pool error: {}", s),
            Self::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            Self::HttpError(e) => write!(f, "HTTP error: {}", e),
            Self::CoreError(e) => write!(f, "Core error: {}", e),
            Self::JobInProgress => write!(f, "Job already in progress"),
        }
    }
}

impl std::error::Error for Error {}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Self::RecordNotFound,
            _ => Self::DbError(error),
        }
    }
}

impl<E: std::fmt::Debug> From<deadpool::managed::PoolError<E>> for Error {
    fn from(error: deadpool::managed::PoolError<E>) -> Self {
        Self::DbPoolError(format!("{:?}", error))
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Self::InvalidUrl(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::HttpError(error)
    }
}

impl From<core_ltx::Error> for Error {
    fn from(error: core_ltx::Error) -> Self {
        Self::CoreError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::RecordNotFound;
        assert_eq!(error.to_string(), "Record not found in database");

        let error = Error::JobInProgress;
        assert_eq!(error.to_string(), "Job already in progress");

        let error = Error::DbPoolError("connection failed".to_string());
        assert_eq!(error.to_string(), "Database pool error: connection failed");
    }

    #[test]
    fn test_error_from_diesel_not_found() {
        let diesel_error = diesel::result::Error::NotFound;
        let error: Error = diesel_error.into();
        assert!(matches!(error, Error::RecordNotFound));
    }

    #[test]
    fn test_error_from_url_parse_error() {
        let url_result = url::Url::parse("not a valid url");
        assert!(url_result.is_err());

        let url_error = url_result.unwrap_err();
        let error: Error = url_error.into();
        assert!(matches!(error, Error::InvalidUrl(_)));
    }
}
