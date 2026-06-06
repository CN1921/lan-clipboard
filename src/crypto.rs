use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use argon2::password_hash::SaltString;
use base64::{Engine as _, engine::general_purpose};
use thiserror::Error;

const KEY_LEN: usize = 32;

/// Cryptographic error types
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    
    #[error("argon2 password hash error: {0}")]
    Argon2Password(#[from] argon2::password_hash::Error),
    
    #[error("argon2 error: {0}")]
    Argon2(#[from] argon2::Error),
    
    #[error("crypto error: {0}")]
    Other(String),
}

/// Derive a cryptographic key from a passphrase using Argon2
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], CryptoError> {
    let mut out = [0u8; KEY_LEN];
    let argon2 = Argon2::default();
    
    argon2.hash_password_into(passphrase.as_bytes(), salt, &mut out)?;
    Ok(out)
}

/// Hash a password and return the hash
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(rand::thread_rng());
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    
    Ok(password_hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(CryptoError::Argon2Password(e)),
    }
}

/// Encode bytes to base64
pub fn encode_base64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Decode base64 to bytes
pub fn decode_base64(data: &str) -> Result<Vec<u8>, CryptoError> {
    Ok(general_purpose::STANDARD.decode(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let passphrase = "test_passphrase";
        let salt = b"test_salt_12345";
        
        let key = derive_key(passphrase, salt);
        assert!(key.is_ok());
        
        let key1 = key.unwrap();
        assert_eq!(key1.len(), KEY_LEN);
    }

    #[test]
    fn test_hash_and_verify_password() {
        let password = "my_secure_password";
        
        let hash = hash_password(password);
        assert!(hash.is_ok());
        
        let hash = hash.unwrap();
        let verify = verify_password(password, &hash);
        assert!(verify.is_ok());
        assert!(verify.unwrap());
        
        let verify_wrong = verify_password("wrong_password", &hash);
        assert!(verify_wrong.is_ok());
        assert!(!verify_wrong.unwrap());
    }

    #[test]
    fn test_base64_encode_decode() {
        let original = b"Hello, World!";
        
        let encoded = encode_base64(original);
        let decoded = decode_base64(&encoded);
        
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), original);
    }
}
