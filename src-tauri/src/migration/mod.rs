use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const NONCE_SIZE: usize = 12;
const SALT_SIZE: usize = 16;
const BACKUP_MAGIC: &[u8] = b"TAURIDRIVE_BKP1"; // Version 1 backup format

/// Backup data structure containing all exportable app data
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub version: u32,
    pub app_version: String,
    pub created_at: String,
    pub credentials: Option<CredentialsBackup>,
    pub sync_folders: Vec<SyncFolderBackup>,
    pub settings: Vec<SettingBackup>,
    pub upload_history: Vec<UploadHistoryBackup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsBackup {
    pub bucket_name: String,
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncFolderBackup {
    pub local_path: String,
    pub remote_path: String,
    pub sync_mode: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingBackup {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadHistoryBackup {
    pub file_path: String,
    pub remote_path: String,
    pub total_size: i64,
    pub status: String,
    pub completed_at: Option<String>,
}

/// Derive encryption key from password using PBKDF2-like approach
fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Multiple rounds of hashing for key derivation
    let mut data = Vec::new();
    data.extend_from_slice(password.as_bytes());
    data.extend_from_slice(salt);
    
    for _ in 0..100_000 {
        hasher.update(&data);
        let result = hasher.finalize_reset();
        data.clear();
        data.extend_from_slice(&result);
        data.extend_from_slice(salt);
    }
    
    let final_hash = hasher.chain_update(&data).finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&final_hash);
    key
}

/// Encrypt backup data with a password
pub fn encrypt_backup(backup: &BackupData, password: &str) -> Result<Vec<u8>> {
    // Serialize backup to JSON
    let json = serde_json::to_string(backup)
        .context("Failed to serialize backup data")?;
    
    // Generate random salt and nonce
    let mut salt = [0u8; SALT_SIZE];
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);
    
    // Derive key from password
    let key = derive_key_from_password(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .context("Failed to create cipher")?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt the JSON data
    let ciphertext = cipher
        .encrypt(nonce, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
    
    // Build final format: MAGIC + SALT + NONCE + CIPHERTEXT
    let mut output = Vec::new();
    output.extend_from_slice(BACKUP_MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    
    Ok(output)
}

/// Decrypt backup data with a password
pub fn decrypt_backup(encrypted: &[u8], password: &str) -> Result<BackupData> {
    // Verify magic header
    if encrypted.len() < BACKUP_MAGIC.len() + SALT_SIZE + NONCE_SIZE {
        return Err(anyhow::anyhow!("Invalid backup file: too short"));
    }
    
    let magic = &encrypted[..BACKUP_MAGIC.len()];
    if magic != BACKUP_MAGIC {
        return Err(anyhow::anyhow!("Invalid backup file: wrong format or version"));
    }
    
    let offset = BACKUP_MAGIC.len();
    let salt = &encrypted[offset..offset + SALT_SIZE];
    let nonce_bytes = &encrypted[offset + SALT_SIZE..offset + SALT_SIZE + NONCE_SIZE];
    let ciphertext = &encrypted[offset + SALT_SIZE + NONCE_SIZE..];
    
    // Derive key from password
    let key = derive_key_from_password(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .context("Failed to create cipher")?;
    let nonce = Nonce::from_slice(nonce_bytes);
    
    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed: incorrect password or corrupted file"))?;
    
    // Parse JSON
    let backup: BackupData = serde_json::from_slice(&plaintext)
        .context("Failed to parse backup data")?;
    
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_backup() -> BackupData {
        BackupData {
            version: 1,
            app_version: "0.1.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            credentials: Some(CredentialsBackup {
                bucket_name: "test-bucket".to_string(),
                account_id: "acc123".to_string(),
                access_key_id: "key123".to_string(),
                secret_access_key: "secret123".to_string(),
                endpoint: "https://test.r2.cloudflarestorage.com".to_string(),
            }),
            sync_folders: vec![
                SyncFolderBackup {
                    local_path: "/home/user/docs".to_string(),
                    remote_path: "docs/".to_string(),
                    sync_mode: "upload_only".to_string(),
                    enabled: true,
                },
            ],
            settings: vec![
                SettingBackup {
                    key: "theme".to_string(),
                    value: "dark".to_string(),
                },
            ],
            upload_history: vec![
                UploadHistoryBackup {
                    file_path: "/home/user/docs/file.txt".to_string(),
                    remote_path: "docs/file.txt".to_string(),
                    total_size: 1024,
                    status: "completed".to_string(),
                    completed_at: Some("2024-01-01T12:00:00Z".to_string()),
                },
            ],
        }
    }

    #[test]
    fn test_encrypt_decrypt_backup() {
        let backup = create_test_backup();
        
        let password = "test-password-123";
        let encrypted = encrypt_backup(&backup, password).unwrap();
        let decrypted = decrypt_backup(&encrypted, password).unwrap();
        
        assert_eq!(decrypted.version, backup.version);
        assert_eq!(decrypted.app_version, backup.app_version);
        assert!(decrypted.credentials.is_some());
        assert_eq!(decrypted.credentials.unwrap().bucket_name, "test-bucket");
    }

    #[test]
    fn test_encrypt_decrypt_with_all_data() {
        let backup = create_test_backup();
        let password = "secure-password-456";
        
        let encrypted = encrypt_backup(&backup, password).unwrap();
        let decrypted = decrypt_backup(&encrypted, password).unwrap();
        
        // Verify all data is preserved
        assert_eq!(decrypted.sync_folders.len(), 1);
        assert_eq!(decrypted.sync_folders[0].local_path, "/home/user/docs");
        assert_eq!(decrypted.sync_folders[0].remote_path, "docs/");
        assert!(decrypted.sync_folders[0].enabled);
        
        assert_eq!(decrypted.settings.len(), 1);
        assert_eq!(decrypted.settings[0].key, "theme");
        assert_eq!(decrypted.settings[0].value, "dark");
        
        assert_eq!(decrypted.upload_history.len(), 1);
        assert_eq!(decrypted.upload_history[0].total_size, 1024);
        assert_eq!(decrypted.upload_history[0].status, "completed");
    }

    #[test]
    fn test_wrong_password() {
        let backup = BackupData {
            version: 1,
            app_version: "0.1.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            credentials: None,
            sync_folders: vec![],
            settings: vec![],
            upload_history: vec![],
        };
        
        let encrypted = encrypt_backup(&backup, "correct-password").unwrap();
        let result = decrypt_backup(&encrypted, "wrong-password");
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("incorrect password"));
    }

    #[test]
    fn test_backup_without_credentials() {
        let backup = BackupData {
            version: 1,
            app_version: "0.1.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            credentials: None,
            sync_folders: vec![],
            settings: vec![],
            upload_history: vec![],
        };
        
        let password = "test-password";
        let encrypted = encrypt_backup(&backup, password).unwrap();
        let decrypted = decrypt_backup(&encrypted, password).unwrap();
        
        assert!(decrypted.credentials.is_none());
    }

    #[test]
    fn test_magic_header_present() {
        let backup = create_test_backup();
        let encrypted = encrypt_backup(&backup, "password").unwrap();
        
        // Check that magic header is at the beginning
        assert!(encrypted.starts_with(BACKUP_MAGIC));
    }

    #[test]
    fn test_invalid_magic_header() {
        let mut invalid_data = b"INVALID_HEADER".to_vec();
        invalid_data.extend_from_slice(&[0u8; 100]); // Pad with zeros
        
        let result = decrypt_backup(&invalid_data, "password");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrong format"));
    }

    #[test]
    fn test_truncated_backup() {
        let backup = create_test_backup();
        let encrypted = encrypt_backup(&backup, "password").unwrap();
        
        // Truncate to just the header
        let truncated = &encrypted[..BACKUP_MAGIC.len() + 10];
        let result = decrypt_backup(truncated, "password");
        
        assert!(result.is_err());
    }

    #[test]
    fn test_different_passwords_different_output() {
        let backup = create_test_backup();
        
        let encrypted1 = encrypt_backup(&backup, "password1").unwrap();
        let encrypted2 = encrypt_backup(&backup, "password2").unwrap();
        
        // Different passwords should produce different ciphertext
        // (Note: Even same password produces different output due to random salt/nonce)
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_same_password_different_output() {
        let backup = create_test_backup();
        let password = "same-password";
        
        let encrypted1 = encrypt_backup(&backup, password).unwrap();
        let encrypted2 = encrypt_backup(&backup, password).unwrap();
        
        // Same password should produce different ciphertext due to random salt/nonce
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt correctly
        let decrypted1 = decrypt_backup(&encrypted1, password).unwrap();
        let decrypted2 = decrypt_backup(&encrypted2, password).unwrap();
        
        assert_eq!(decrypted1.version, decrypted2.version);
    }

    #[test]
    fn test_empty_password() {
        let backup = create_test_backup();
        let password = "";
        
        // Empty password should still work (though not recommended)
        let encrypted = encrypt_backup(&backup, password).unwrap();
        let decrypted = decrypt_backup(&encrypted, password).unwrap();
        
        assert_eq!(decrypted.version, backup.version);
    }

    #[test]
    fn test_unicode_password() {
        let backup = create_test_backup();
        let password = "密码🔐パスワード";
        
        let encrypted = encrypt_backup(&backup, password).unwrap();
        let decrypted = decrypt_backup(&encrypted, password).unwrap();
        
        assert_eq!(decrypted.version, backup.version);
    }

    #[test]
    fn test_long_password() {
        let backup = create_test_backup();
        let password = "a".repeat(1000); // Very long password
        
        let encrypted = encrypt_backup(&backup, &password).unwrap();
        let decrypted = decrypt_backup(&encrypted, &password).unwrap();
        
        assert_eq!(decrypted.version, backup.version);
    }

    #[test]
    fn test_key_derivation_consistency() {
        let password = "test-password";
        let salt = [0u8; SALT_SIZE]; // Fixed salt for testing
        
        let key1 = derive_key_from_password(password, &salt);
        let key2 = derive_key_from_password(password, &salt);
        
        // Same password and salt should produce same key
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_derivation_salt_sensitivity() {
        let password = "test-password";
        let salt1 = [0u8; SALT_SIZE];
        let salt2 = [1u8; SALT_SIZE];
        
        let key1 = derive_key_from_password(password, &salt1);
        let key2 = derive_key_from_password(password, &salt2);
        
        // Different salt should produce different key
        assert_ne!(key1, key2);
    }
}
