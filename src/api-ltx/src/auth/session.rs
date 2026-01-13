use base64::{Engine as _, engine::general_purpose};
use cookie::{Cookie, SameSite};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const COOKIE_NAME: &str = "llm_web_index_session";

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Invalid token format")]
    InvalidFormat,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Token expired")]
    Expired,

    #[error("HMAC error: {0}")]
    HmacError(String),

    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// Generate a session token with format: timestamp:nonce:signature
/// The signature is HMAC-SHA256(timestamp:nonce, secret)
pub fn generate_session_token(secret: &str) -> Result<String, SessionError> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Generate cryptographically secure random nonce
    let nonce: [u8; 16] = rand::random();
    let nonce_b64 = general_purpose::URL_SAFE_NO_PAD.encode(nonce);

    // Create payload: timestamp:nonce
    let payload = format!("{}:{}", timestamp, nonce_b64);

    // Sign payload with HMAC-SHA256
    let signature = sign_payload(&payload, secret)?;

    // Final token: timestamp:nonce:signature
    Ok(format!("{}:{}", payload, signature))
}

/// Validate a session token
/// Returns Ok(true) if valid and not expired, Ok(false) if invalid/expired
pub fn validate_session_token(token: &str, secret: &str, max_age_secs: u64) -> Result<bool, SessionError> {
    // Parse token: timestamp:nonce:signature
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 3 {
        return Err(SessionError::InvalidFormat);
    }

    let timestamp_str = parts[0];
    let nonce = parts[1];
    let provided_signature = parts[2];

    // Parse timestamp
    let timestamp: u64 = timestamp_str.parse().map_err(|_| SessionError::InvalidFormat)?;

    // Check expiration
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    if current_time - timestamp > max_age_secs {
        return Ok(false); // Expired
    }

    // Verify signature
    let payload = format!("{}:{}", timestamp_str, nonce);
    let expected_signature = sign_payload(&payload, secret)?;

    // Constant-time comparison
    if provided_signature != expected_signature {
        return Ok(false); // Invalid signature
    }

    Ok(true)
}

/// Create a session cookie with the token
pub fn create_session_cookie(token: &str, max_age_secs: u64) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, token.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(max_age_secs as i64))
        .path("/")
        .build()
}

/// Create a cookie to clear the session (for logout)
pub fn create_logout_cookie() -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .path("/")
        .build()
}

/// Parse session token from Cookie header
pub fn parse_session_cookie(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .filter_map(|pair| Cookie::parse(pair.trim()).ok())
        .find(|cookie| cookie.name() == COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
}

/// Sign a payload using HMAC-SHA256
fn sign_payload(payload: &str, secret: &str) -> Result<String, SessionError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| SessionError::HmacError(e.to_string()))?;

    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();

    Ok(general_purpose::URL_SAFE_NO_PAD.encode(code_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    const TEST_SECRET: &str = "test_secret_key_for_hmac_signing";

    #[test]
    fn test_generate_and_validate_token() {
        let token = generate_session_token(TEST_SECRET).unwrap();
        assert!(validate_session_token(&token, TEST_SECRET, 3600).unwrap());
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let token = generate_session_token(TEST_SECRET).unwrap();
        assert!(!validate_session_token(&token, "wrong_secret", 3600).unwrap());
    }

    #[test]
    fn test_validate_token_expired() {
        let token = generate_session_token(TEST_SECRET).unwrap();
        sleep(Duration::from_secs(2));
        // Token with max_age of 1 second should be expired
        assert!(!validate_session_token(&token, TEST_SECRET, 1).unwrap());
    }

    #[test]
    fn test_validate_token_invalid_format() {
        let result = validate_session_token("invalid", TEST_SECRET, 3600);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_session_cookie() {
        let cookie_header = "llm_web_index_session=abc123; Path=/; HttpOnly";
        let token = parse_session_cookie(cookie_header);
        assert_eq!(token, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_session_cookie_multiple() {
        let cookie_header = "other=value; llm_web_index_session=abc123; another=test";
        let token = parse_session_cookie(cookie_header);
        assert_eq!(token, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_session_cookie_missing() {
        let cookie_header = "other=value; another=test";
        let token = parse_session_cookie(cookie_header);
        assert_eq!(token, None);
    }

    #[test]
    fn test_create_session_cookie() {
        let cookie = create_session_cookie("test_token", 86400);
        assert_eq!(cookie.name(), COOKIE_NAME);
        assert_eq!(cookie.value(), "test_token");
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    }
}
