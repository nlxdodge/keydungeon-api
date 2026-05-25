use crate::config::BcryptConfig;

#[derive(Debug)]
pub enum PasswordError {
    HashingFailed,
    VerificationFailed,
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordError::HashingFailed => write!(f, "Failed to hash password"),
            PasswordError::VerificationFailed => write!(f, "Password verification failed"),
        }
    }
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let bcrypt_config = BcryptConfig::from_env();
    bcrypt::hash(password, bcrypt_config.cost).map_err(|_| PasswordError::HashingFailed)
}

/// Verify a password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    bcrypt::verify(password, hash).map_err(|_| PasswordError::VerificationFailed)
}
