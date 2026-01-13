use std::{env::VarError, num::ParseIntError};

/// Same as api_base_url but panics on error.
pub fn get_api_base_url() -> String {
    api_base_url().expect("Invalid HOST or PORT")
}

/// Gets the host:port from the env vars HOST and PORT.
/// PORT defaults to `3000` if not presnet but HOST is required.
pub fn api_base_url() -> Result<String, HostPortError> {
    let host = std::env::var("HOST").and_then(|h| {
        let h = h.trim().to_string();
        if h.is_empty() { Err(VarError::NotPresent) } else { Ok(h) }
    })?;
    let port = match std::env::var("PORT") {
        Ok(p) => p.parse::<u16>()?,
        Err(_) => 3000,
    };
    let address = format!("{}:{}", host, port);
    Ok(address)
}

#[derive(Debug)]
pub enum HostPortError {
    NoHostEnv,
    InvalidPort(ParseIntError),
    InvalidHostname(std::net::AddrParseError),
}

impl std::error::Error for HostPortError {}

impl std::fmt::Display for HostPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostPortError::NoHostEnv => write!(f, "Missing required non-empty HOST environment variable!"),
            HostPortError::InvalidPort(err) => write!(f, "Invalid port: {}", err),
            HostPortError::InvalidHostname(err) => write!(f, "Invalid hostname: {}", err),
        }
    }
}

impl From<VarError> for HostPortError {
    fn from(_: VarError) -> Self {
        HostPortError::NoHostEnv
    }
}

impl From<ParseIntError> for HostPortError {
    fn from(err: ParseIntError) -> Self {
        HostPortError::InvalidPort(err)
    }
}

impl From<std::net::AddrParseError> for HostPortError {
    fn from(err: std::net::AddrParseError) -> Self {
        HostPortError::InvalidHostname(err)
    }
}
