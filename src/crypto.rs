//! Crypto utilities for lan-clipboard
//!
//! - Key derivation using Argon2
//! - AEAD encryption using XChaCha20-Poly1305
//! - Base64 encoding for transport

use argon2::{Argon2, Params};
use chacha20poly1305::{aead::{Aead, KeyInit}, XChaCha20Poly1305, XNonce, Key};
use rand::RngCore;
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

pub const SALT_LEN: usize = 16;
pub const KEY_LEN: usize = 32; // 256-bit key
pub const NONCE_LEN: usize = 24; // XChaCha20-Poly1305 nonce size

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("argon2 error: {0}")]
    Argon2(#[from] argon2::password_hash::Error),
    #[error("encryption error")]
    Encryption,
    #[error("decryption error")]
    Decryption,
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Generate a cryptographically-random salt
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 32-byte key from a passphrase and salt using Argon2id
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], CryptoError> {
    // Use conservative but reasonable parameters. These can be tuned later.
    let params = Params::new(4096, 3, 1, None).map_err(|e| argon2::password_hash::Error::from(e))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut out = [0u8; KEY_LEN];
    // hash_password_into signature takes password bytes and salt bytes
    argon2.hash_password_into(passphrase.as_bytes(), salt, &mut out)?;
    Ok(out)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncryptedMessage {
    /// Base64-encoded nonce
    pub nonce_b64: String,
    /// Base64-encoded ciphertext (includes tag)
    pub ciphertext_b64: String,
}

/// Encrypt plaintext with the given 32-byte key. Returns base64-encoded nonce and ciphertext.
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<EncryptedMessage, CryptoError> {
    let aead_key = Key::from_slice(key);
    let cipher = XChaCha20Poly1305::new(aead_key);

    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let nonce_struct = XNonce::from_slice(&nonce);

    let ciphertext = cipher
        .encrypt(nonce_struct, plaintext)
        .map_err(|_| CryptoError::Encryption)?;

    Ok(EncryptedMessage {
        nonce_b64: general_purpose::STANDARD.encode(&nonce),
        ciphertext_b64: general_purpose::STANDARD.encode(&ciphertext),
    })
}

/// Decrypt an EncryptedMessage with the given key
pub fn decrypt(key: &[u8; KEY_LEN], msg: &EncryptedMessage) -> Result<Vec<u8>, CryptoError> {
    let aead_key = Key::from_slice(key);
    let cipher = XChaCha20Poly1305::new(aead_key);

    let nonce = general_purpose::STANDARD.decode(&msg.nonce_b64)?;
    if nonce.len() != NONCE_LEN {
        return Err(CryptoError::Decryption);
    }
    let nonce_struct = XNonce::from_slice(&nonce);

    let ciphertext = general_purpose::STANDARD.decode(&msg.ciphertext_b64)?;
    let plaintext = cipher
        .decrypt(nonce_struct, ciphertext.as_ref())
        .map_err(|_| CryptoError::Decryption)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_consistency() {
        let pass = "correct horse battery staple";
        let salt = generate_salt();
        let k1 = derive_key(pass, &salt).expect("derive");
        let k2 = derive_key(pass, &salt).expect("derive");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let pass = "hunter2";
        let salt = generate_salt();
        let key = derive_key(pass, &salt).expect("derive");
        let plaintext = b"hello lan clipboard";

        let enc = encrypt(&key, plaintext).expect("encrypt");
        let dec = decrypt(&key, &enc).expect("decrypt");
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn test_decrypt_failure_wrong_key() {
        let pass = "abc";
        let salt = generate_salt();
        let key = derive_key(pass, &salt).expect("derive");
        let plaintext = b"secret";
        let enc = encrypt(&key, plaintext).expect("encrypt");

        let other_key = derive_key("other", &salt).expect("derive");
        let res = decrypt(&other_key, &enc);
        assert!(res.is_err());
    }
}
