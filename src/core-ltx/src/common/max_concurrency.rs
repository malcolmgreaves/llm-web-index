use std::env::VarError;
use std::num::ParseIntError;

/// The default maximum concurrency value.
pub const DEFAULT: usize = 1000;

/// Same as max_concurrency but panics on error.
pub fn get_max_concurrency(override_default: Option<usize>) -> usize {
    match max_concurrency() {
        Ok(v) => v,
        Err(MaxConcurrencyError::MissingEnvVar(_)) => override_default.unwrap_or(DEFAULT),
        _ => panic!("WORKER_MAX_CONCURRENCY must be a valid positive number"),
    }
}

/// Retrieves the value of the environment variable as a usize for max concurrency.
/// Uses `usize` because the intended use of this value is in a semaphore, which requires a usize.
pub fn max_concurrency() -> Result<usize, MaxConcurrencyError> {
    std::env::var("WORKER_MAX_CONCURRENCY")
        .map_err(|e| e.into())
        .and_then(|v| v.trim().parse::<usize>().map_err(|e| e.into()))
}

#[derive(Debug)]
pub enum MaxConcurrencyError {
    ParseIntError(ParseIntError),
    NonPositive,
    MissingEnvVar(VarError),
}

impl std::error::Error for MaxConcurrencyError {}

impl From<ParseIntError> for MaxConcurrencyError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseIntError(error)
    }
}

impl From<VarError> for MaxConcurrencyError {
    fn from(error: VarError) -> Self {
        Self::MissingEnvVar(error)
    }
}

impl std::fmt::Display for MaxConcurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::ParseIntError(e) => write!(f, "Failed to parse environment variable value as an integer: {}", e),
            Self::NonPositive => write!(f, "WORKER_MAX_CONCURRENCY must be a positive number"),
            Self::MissingEnvVar(e) => write!(f, "Environment variable WORKER_MAX_CONCURRENCY is missing: {}", e),
        }
    }
}
