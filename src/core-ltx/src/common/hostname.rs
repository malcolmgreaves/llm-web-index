use std::{net::SocketAddr, num::ParseIntError};

/// Gets the host:port from the env vars HOST and PORT.
/// Uses defaults `127.0.0.1:3000` if env vars are empty.
pub fn get_api_base_url() -> Result<SocketAddr, HostPortError> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = match std::env::var("PORT") {
        Ok(p) => p.parse::<u16>()?,
        Err(_) => 3000,
    };
    let address = format!("{}:{}", host, port).parse::<SocketAddr>()?;
    Ok(address)
}

#[derive(Debug)]
pub enum HostPortError {
    InvalidPort(ParseIntError),
    InvalidHostname(std::net::AddrParseError),
}

impl std::error::Error for HostPortError {}

impl std::fmt::Display for HostPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostPortError::InvalidPort(err) => write!(f, "Invalid port: {}", err),
            HostPortError::InvalidHostname(err) => write!(f, "Invalid hostname: {}", err),
        }
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
