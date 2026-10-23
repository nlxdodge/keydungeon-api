use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use password_hash::phc::SaltString;

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
pub fn hash(password: &str) -> Result<String, PasswordError> {
    let argon2 = Argon2::default();
    let salt = SaltString::generate();

    let argon = Argon2::new_with_secret(
        b"secret pepper",
        Algorithm::default(),
        Version::default(),
        Params::default()
    );

    
    let hash = argon2
        .hash_password_with_salt(password.as_bytes(), &salt.as_bytes())
        .map_err(|_| PasswordError::HashingFailed);
        
    Ok("test")
}

/// Verify a password against a bcrypt hash
pub fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| PasswordError::VerificationFailed)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
