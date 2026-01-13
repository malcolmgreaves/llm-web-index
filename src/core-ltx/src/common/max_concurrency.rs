use std::num::ParseIntError;

/// Same as max_concurrency but panics on error.
pub fn get_max_concurrency(env_var_name: &str, default: usize) -> usize {
    max_concurrency(env_var_name, default)
        .unwrap_or_else(|_| panic!("{} must be a valid positive number", env_var_name))
}

/// Retrieves the value of the environment variable as a usize for max concurrency.
pub fn max_concurrency(env_var_name: &str, default: usize) -> Result<usize, ParseIntError> {
    let max_concurrency = match std::env::var(env_var_name) {
        Ok(v) => v.trim().parse::<usize>()?,
        Err(_) => default,
    };
    Ok(max_concurrency)
}
