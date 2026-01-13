use std::env;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub password_hash: String,
    pub session_secret: String,
    pub session_duration_seconds: u64,
    /// Plain text password for programmatic authentication (e.g., cron service)
    /// Only populated when AUTH_PASSWORD is set
    pub password: Option<String>,
}

/// Check if authentication is enabled
/// True if the env var ENABLE_AUTH is present and is one of "1", "true", "yes", or "y".
/// False otherwise.
pub fn is_auth_enabled() -> bool {
    env::var("ENABLE_AUTH")
        .map(|v| {
            let v = v.trim().to_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "y"
        })
        .unwrap_or(false)
}

/// Get authentication configuration
/// Returns None if authentication is disabled
/// Panics if authentication is enabled but required configuration is missing
pub fn get_auth_config() -> Option<AuthConfig> {
    if !is_auth_enabled() {
        return None;
    }

    let password_hash = env::var("AUTH_PASSWORD_HASH").expect(
        "AUTH_PASSWORD_HASH environment variable is required when ENABLE_AUTH=true. \
         Generate a hash with: cargo run --bin generate-password-hash -- your_password",
    );

    let session_secret = env::var("SESSION_SECRET").expect(
        "SESSION_SECRET environment variable is required when ENABLE_AUTH=true. \
         Generate a secret with: openssl rand -base64 32",
    );

    let session_duration_seconds = env::var("SESSION_DURATION_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(86400); // Default: 24 hours

    let password = env::var("AUTH_PASSWORD").ok();

    Some(AuthConfig {
        password_hash,
        session_secret,
        session_duration_seconds,
        password,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Use a mutex to ensure tests that modify env vars run serially
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_is_auth_enabled_default() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            env::remove_var("ENABLE_AUTH");
        }
        assert!(!is_auth_enabled());
    }

    #[test]
    fn test_is_auth_enabled_true() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("ENABLE_AUTH", "true");
        }
        assert!(is_auth_enabled());
        unsafe {
            env::remove_var("ENABLE_AUTH");
        }
    }

    #[test]
    fn test_is_auth_enabled_1() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("ENABLE_AUTH", "1");
        }
        assert!(is_auth_enabled());
        unsafe {
            env::remove_var("ENABLE_AUTH");
        }
    }

    #[test]
    fn test_is_auth_enabled_false() {
        let _guard = TEST_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("ENABLE_AUTH", "false");
        }
        assert!(!is_auth_enabled());
        unsafe {
            env::remove_var("ENABLE_AUTH");
        }
    }
}
