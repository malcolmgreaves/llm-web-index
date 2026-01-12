use std::{num::ParseIntError, time::Duration};

/// Retrieves the value of the environment variable as a duration.
pub fn get_poll_interval(units: TimeUnit, env_var_name: &str, default: u64) -> Result<Duration, ParseIntError> {
    let poll_interval = match std::env::var(env_var_name) {
        Ok(v) => v.trim().parse::<u64>()?,
        Err(_) => default,
    };

    Ok(match units {
        TimeUnit::Seconds => Duration::from_secs(poll_interval),
        TimeUnit::Milliseconds => Duration::from_millis(poll_interval),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
}
