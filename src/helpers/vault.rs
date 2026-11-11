use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use chacha20poly1305::aead::{Aead, Generate, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Debug)]
pub enum VaultError {
    MissingKey,
    InvalidKey,
    InvalidEncoding,
    InvalidCiphertext,
    EncryptFailed,
    DecryptFailed,
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultError::MissingKey => write!(f, "VAULT_KEY is not set"),
            VaultError::InvalidKey => write!(f, "VAULT_KEY must be a base64 encoded 32-byte key"),
            VaultError::InvalidEncoding => write!(f, "Vault payload is not valid base64"),
            VaultError::InvalidCiphertext => write!(f, "Vault payload is too short"),
            VaultError::EncryptFailed => write!(f, "Failed to encrypt vault entry"),
            VaultError::DecryptFailed => write!(f, "Failed to decrypt vault entry"),
        }
    }
}

impl std::error::Error for VaultError {}

/// Encrypts and decrypts vault entries with ChaCha20-Poly1305.
///
/// The base64-encoded key comes from `VAULT_KEY`. Every encryption uses a
/// fresh random 96-bit nonce, which is prepended to the ciphertext before the
/// combined payload is stored as base64 in SQLite.
#[derive(Clone)]
pub struct Vault {
    cipher: ChaCha20Poly1305,
}

impl Vault {
    /// Build a vault from the `VAULT_KEY` environment variable.
    pub fn from_env() -> Result<Self, VaultError> {
        let encoded =
            SecretString::from(std::env::var("VAULT_KEY").map_err(|_| VaultError::MissingKey)?);

        Self::from_base64(encoded.expose_secret())
    }

    /// Build a vault from a base64-encoded 32-byte key.
    pub fn from_base64(encoded: &str) -> Result<Self, VaultError> {
        let decoded = Zeroizing::new(
            STANDARD
                .decode(encoded.trim())
                .map_err(|_| VaultError::InvalidEncoding)?,
        );
        let key = Key::try_from(decoded.as_slice()).map_err(|_| VaultError::InvalidKey)?;

        Ok(Self {
            cipher: ChaCha20Poly1305::new(&key),
        })
    }

    /// Encrypt a plaintext vault entry.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, VaultError> {
        let nonce = Nonce::generate();
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| VaultError::EncryptFailed)?;

        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(nonce.as_slice());
        payload.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(payload))
    }

    /// Decrypt a base64 vault entry. The plaintext is zeroized when dropped.
    pub fn decrypt(&self, encoded: &str) -> Result<Zeroizing<String>, VaultError> {
        let payload = Zeroizing::new(
            STANDARD
                .decode(encoded.trim())
                .map_err(|_| VaultError::InvalidEncoding)?,
        );

        if payload.len() < NONCE_LEN + TAG_LEN {
            return Err(VaultError::InvalidCiphertext);
        }

        let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);
        let nonce = Nonce::try_from(nonce_bytes).map_err(|_| VaultError::InvalidCiphertext)?;
        let plaintext = Zeroizing::new(
            self.cipher
                .decrypt(&nonce, ciphertext)
                .map_err(|_| VaultError::DecryptFailed)?,
        );
        let plaintext = std::str::from_utf8(plaintext.as_slice())
            .map_err(|_| VaultError::DecryptFailed)?
            .to_string();

        Ok(Zeroizing::new(plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::Vault;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    fn test_vault() -> Vault {
        let key = STANDARD.encode([7_u8; 32]);
        Vault::from_base64(&key).expect("test key should be valid")
    }

    #[test]
    fn encrypts_and_decrypts_an_entry() {
        let vault = test_vault();
        let encrypted = vault.encrypt("super-secret-password").unwrap();

        assert_ne!(encrypted, "super-secret-password");
        assert_eq!(
            vault.decrypt(&encrypted).unwrap().as_str(),
            "super-secret-password"
        );
    }

    #[test]
    fn rejects_tampered_ciphertext() {
        let vault = test_vault();
        let encrypted = vault.encrypt("super-secret-password").unwrap();
        let mut bytes = STANDARD.decode(encrypted).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;

        assert!(vault.decrypt(&STANDARD.encode(bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_key_length() {
        assert!(Vault::from_base64(&STANDARD.encode([7_u8; 16])).is_err());
    }
}
