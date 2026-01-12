use std::{num::ParseIntError, time::Duration};

/// Same as poll_interval but panics on error.
pub fn get_poll_interval(units: TimeUnit, env_var_name: &str, default: u64) -> Duration {
    poll_interval(units, env_var_name, default).expect(format!("{} must be a valid number", env_var_name).as_str())
}

#[derive(Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
}

/// Retrieves the value of the environment variable as a duration.
pub fn poll_interval(units: TimeUnit, env_var_name: &str, default: u64) -> Result<Duration, ParseIntError> {
    let polling = match std::env::var(env_var_name) {
        Ok(v) => v.trim().parse::<u64>()?,
        Err(_) => default,
    };

    let interval = match units {
        TimeUnit::Seconds => Duration::from_secs(polling),
        TimeUnit::Milliseconds => Duration::from_millis(polling),
    };
    Ok(interval)
}
