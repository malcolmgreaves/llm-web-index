use std::num::ParseIntError;

pub const DEFAULT: u32 = 1000;

#[derive(Debug)]
pub enum MaxConcurrencyError {
    ParseIntError(ParseIntError),
    NonPositive,
}

impl std::error::Error for MaxConcurrencyError {}

impl From<ParseIntError> for MaxConcurrencyError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseIntError(error)
    }
}

impl std::fmt::Display for MaxConcurrencyError {
}



/// Same as max_concurrency but panics on error.
pub fn get_max_concurrency(env_var_name: &str, default: Option<u32>) -> u32 {
    max_concurrency(env_var_name, default.unwrap_or(DEFAULT))
        .unwrap_or_else(|_| panic!("{} must be a valid positive number", env_var_name))
}

/// Retrieves the value of the environment variable as a usize for max concurrency.
pub fn max_concurrency(env_var_name: &str) -> Result<u32, MaxConcurrencyError> {
    let max_concurrency = match std::env::var(env_var_name) {
        Ok(v) => v.trim().parse::<u32>()?,
        Err(_) => default,
    };
    
    Ok(max_concurrency)
}
