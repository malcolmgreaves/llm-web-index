use bcrypt;

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("Bcrypt verification failed: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
}

/// Verify a password against a bcrypt hash
/// Uses constant-time comparison to prevent timing attacks
pub fn verify_password(plaintext: &str, hash: &str) -> Result<bool, PasswordError> {
    bcrypt::verify(plaintext, hash).map_err(PasswordError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_password_correct() {
        // Generate a hash for "test_password" and verify it
        let hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
        assert!(verify_password("test_password", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        // Generate a hash for "test_password" but try wrong password
        let hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("test_password", "invalid_hash");
        assert!(result.is_err());
    }
}
