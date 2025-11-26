use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const NONCE_SIZE: usize = 12;
const KEY_FILE_NAME: &str = ".tauri-drive-key";

/// Crypto module for encrypting/decrypting credentials
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    /// Create a new Crypto instance, loading or generating the encryption key
    pub fn new() -> Result<Self> {
        let key = Self::load_or_create_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .context("Failed to create cipher from key")?;
        Ok(Self { cipher })
    }

    /// Get the path to the key file
    fn key_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("tauri-drive");
        path.push(KEY_FILE_NAME);
        path
    }

    /// Load existing key or create a new one
    fn load_or_create_key() -> Result<[u8; 32]> {
        let key_path = Self::key_path();
        
        if key_path.exists() {
            // Load existing key
            let key_data = std::fs::read(&key_path)
                .context("Failed to read encryption key")?;
            let decoded = BASE64.decode(&key_data)
                .context("Failed to decode encryption key")?;
            
            let mut key = [0u8; 32];
            if decoded.len() != 32 {
                return Err(anyhow::anyhow!("Invalid key length"));
            }
            key.copy_from_slice(&decoded);
            Ok(key)
        } else {
            // Generate new key
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            
            // Ensure directory exists
            if let Some(parent) = key_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            // Save key (base64 encoded)
            let encoded = BASE64.encode(&key);
            std::fs::write(&key_path, encoded)
                .context("Failed to save encryption key")?;
            
            // Set restrictive permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&key_path)?.permissions();
                perms.set_mode(0o600); // Owner read/write only
                std::fs::set_permissions(&key_path, perms)?;
            }
            
            Ok(key)
        }
    }

    /// Encrypt a string value
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Combine nonce + ciphertext and encode as base64
        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// Decrypt an encrypted string value
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        // Decode from base64
        let combined = BASE64.decode(encrypted)
            .context("Failed to decode encrypted data")?;

        if combined.len() < NONCE_SIZE {
            return Err(anyhow::anyhow!("Encrypted data too short"));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .context("Decrypted data is not valid UTF-8")
    }

    /// Hash a value (for non-reversible storage if needed)
    pub fn hash(value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        let result = hasher.finalize();
        BASE64.encode(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = Crypto::new().unwrap();
        let original = "my-secret-access-key-12345";
        
        let encrypted = crypto.encrypt(original).unwrap();
        assert_ne!(encrypted, original);
        
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_different_nonces() {
        let crypto = Crypto::new().unwrap();
        let original = "test-value";
        
        let encrypted1 = crypto.encrypt(original).unwrap();
        let encrypted2 = crypto.encrypt(original).unwrap();
        
        // Same plaintext should produce different ciphertext due to random nonce
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same value
        assert_eq!(crypto.decrypt(&encrypted1).unwrap(), original);
        assert_eq!(crypto.decrypt(&encrypted2).unwrap(), original);
    }

    #[test]
    fn test_empty_string() {
        let crypto = Crypto::new().unwrap();
        let original = "";
        
        let encrypted = crypto.encrypt(original).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_unicode_strings() {
        let crypto = Crypto::new().unwrap();
        let original = "日本語テスト 🔐 密码测试";
        
        let encrypted = crypto.encrypt(original).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_long_string() {
        let crypto = Crypto::new().unwrap();
        let original = "a".repeat(10000);
        
        let encrypted = crypto.encrypt(&original).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_special_characters() {
        let crypto = Crypto::new().unwrap();
        let original = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~\n\t\r";
        
        let encrypted = crypto.encrypt(original).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let crypto = Crypto::new().unwrap();
        
        // Invalid base64
        let result = crypto.decrypt("not-valid-base64!!!");
        assert!(result.is_err());
        
        // Too short (less than nonce size)
        let short_data = BASE64.encode(&[1, 2, 3]);
        let result = crypto.decrypt(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_deterministic() {
        let value = "test-value";
        
        let hash1 = Crypto::hash(value);
        let hash2 = Crypto::hash(value);
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_values() {
        let hash1 = Crypto::hash("value1");
        let hash2 = Crypto::hash("value2");
        
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_not_reversible() {
        let original = "secret-value";
        let hash = Crypto::hash(original);
        
        // Hash should be different from original
        assert_ne!(hash, original);
        
        // Hash should be base64 encoded
        assert!(BASE64.decode(&hash).is_ok());
    }

    #[test]
    fn test_encrypted_is_base64() {
        let crypto = Crypto::new().unwrap();
        let encrypted = crypto.encrypt("test").unwrap();
        
        // Should be valid base64
        let decoded = BASE64.decode(&encrypted);
        assert!(decoded.is_ok());
        
        // Should contain nonce (12 bytes) + ciphertext (at least 16 bytes for tag)
        let data = decoded.unwrap();
        assert!(data.len() >= NONCE_SIZE + 16);
    }

    #[test]
    fn test_consistent_decryption() {
        let crypto = Crypto::new().unwrap();
        let original = "consistent-test";
        
        let encrypted = crypto.encrypt(original).unwrap();
        
        // Decrypt multiple times should give same result
        for _ in 0..5 {
            let decrypted = crypto.decrypt(&encrypted).unwrap();
            assert_eq!(decrypted, original);
        }
    }

    #[test]
    fn test_credentials_encryption() {
        let crypto = Crypto::new().unwrap();
        
        // Test realistic credential values
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        
        let encrypted_access = crypto.encrypt(access_key).unwrap();
        let encrypted_secret = crypto.encrypt(secret_key).unwrap();
        
        assert_eq!(crypto.decrypt(&encrypted_access).unwrap(), access_key);
        assert_eq!(crypto.decrypt(&encrypted_secret).unwrap(), secret_key);
    }
}
