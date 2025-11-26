use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use anyhow::Result;
use std::path::PathBuf;
use crate::crypto::Crypto;

pub struct Database {
    pool: Pool<Sqlite>,
    crypto: Crypto,
}

impl Database {
    pub async fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
            // Keep using tauri-drive for backward compatibility with existing data
            path.push("tauri-drive");
            path.push("app.db");
            path
        });

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db_url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations
        let migrations = include_str!("../../migrations/001_init.sql");
        sqlx::query(migrations).execute(&pool).await?;

        // Run migration to add created_at column if it doesn't exist
        // SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we check first
        let _ = sqlx::query("ALTER TABLE sync_folders ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP")
            .execute(&pool)
            .await; // Ignore error if column already exists

        // Initialize crypto for credential encryption
        let crypto = Crypto::new()?;

        Ok(Self { pool, crypto })
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn save_credentials(
        &self,
        bucket_name: &str,
        account_id: &str,
        access_key_id: &str,
        secret_access_key: &str,
        endpoint: &str,
    ) -> Result<i64> {
        // Encrypt sensitive credentials before storing
        let encrypted_access_key = self.crypto.encrypt(access_key_id)?;
        let encrypted_secret_key = self.crypto.encrypt(secret_access_key)?;
        
        let result = sqlx::query(
            "INSERT INTO buckets (name, account_id, access_key_id, secret_access_key, endpoint)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET
                account_id = excluded.account_id,
                access_key_id = excluded.access_key_id,
                secret_access_key = excluded.secret_access_key,
                endpoint = excluded.endpoint"
        )
        .bind(bucket_name)
        .bind(account_id)
        .bind(&encrypted_access_key)
        .bind(&encrypted_secret_key)
        .bind(endpoint)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn load_credentials(&self) -> Result<Option<(String, String, String, String, String)>> {
        let result = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT name, account_id, access_key_id, secret_access_key, endpoint FROM buckets ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        // Decrypt credentials if found
        if let Some((name, account_id, encrypted_access_key, encrypted_secret_key, endpoint)) = result {
            let access_key_id = self.crypto.decrypt(&encrypted_access_key)?;
            let secret_access_key = self.crypto.decrypt(&encrypted_secret_key)?;
            Ok(Some((name, account_id, access_key_id, secret_access_key, endpoint)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_current_bucket(&self) -> Result<Option<String>> {
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM buckets ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(name,)| name))
    }

    /// Get all sync folders
    pub async fn get_sync_folders(&self) -> Result<Vec<crate::SyncFolder>> {
        // First, get the current bucket id
        let bucket_id: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM buckets ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        let bucket_id = match bucket_id {
            Some((id,)) => id,
            None => return Ok(Vec::new()),
        };

        let result = sqlx::query_as::<_, (i64, String, String, bool, Option<String>)>(
            "SELECT id, local_path, remote_path, enabled, last_sync 
             FROM sync_folders 
             WHERE bucket_id = ? 
             ORDER BY id DESC"
        )
        .bind(bucket_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(result.into_iter().map(|(id, local_path, remote_path, enabled, last_sync)| {
            crate::SyncFolder {
                id,
                local_path,
                remote_path,
                enabled,
                last_sync,
            }
        }).collect())
    }

    /// Add a new sync folder
    pub async fn add_sync_folder(&self, local_path: &str, remote_path: &str) -> Result<i64> {
        // Get the current bucket id
        let bucket_id: (i64,) = sqlx::query_as(
            "SELECT id FROM buckets ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_one(&self.pool)
        .await?;

        let result = sqlx::query(
            "INSERT INTO sync_folders (bucket_id, local_path, remote_path, sync_mode, enabled)
             VALUES (?, ?, ?, 'upload_only', 1)"
        )
        .bind(bucket_id.0)
        .bind(local_path)
        .bind(remote_path)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Remove a sync folder
    pub async fn remove_sync_folder(&self, folder_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM sync_folders WHERE id = ?")
            .bind(folder_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Toggle sync folder enabled status
    pub async fn toggle_sync_folder(&self, folder_id: i64, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE sync_folders SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(folder_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(Some(db_path)).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_load_credentials() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // Save credentials
        let result = db.save_credentials(
            "test-bucket",
            "account123",
            "access_key_id",
            "secret_access_key",
            "https://test.r2.cloudflarestorage.com"
        ).await;
        
        assert!(result.is_ok());
        
        // Load credentials
        let loaded = db.load_credentials().await.unwrap();
        assert!(loaded.is_some());
        
        let (bucket, account_id, access_key, secret_key, endpoint) = loaded.unwrap();
        assert_eq!(bucket, "test-bucket");
        assert_eq!(account_id, "account123");
        assert_eq!(access_key, "access_key_id");
        assert_eq!(secret_key, "secret_access_key");
        assert_eq!(endpoint, "https://test.r2.cloudflarestorage.com");
    }

    #[tokio::test]
    async fn test_update_credentials() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // Save initial credentials
        db.save_credentials(
            "bucket1",
            "account1",
            "key1",
            "secret1",
            "https://endpoint1.com"
        ).await.unwrap();
        
        // Update with same bucket name (UPSERT)
        db.save_credentials(
            "bucket1",
            "account2",
            "key2",
            "secret2",
            "https://endpoint2.com"
        ).await.unwrap();
        
        // Should have updated, not inserted
        let loaded = db.load_credentials().await.unwrap().unwrap();
        assert_eq!(loaded.1, "account2");
        assert_eq!(loaded.2, "key2");
    }

    #[tokio::test]
    async fn test_get_current_bucket() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // No bucket initially
        let bucket = db.get_current_bucket().await.unwrap();
        assert!(bucket.is_none());
        
        // Add a bucket
        db.save_credentials(
            "my-bucket",
            "account",
            "key",
            "secret",
            "https://endpoint.com"
        ).await.unwrap();
        
        let bucket = db.get_current_bucket().await.unwrap();
        assert_eq!(bucket, Some("my-bucket".to_string()));
    }

    #[tokio::test]
    async fn test_sync_folders_crud() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // First add a bucket (required for sync folders)
        db.save_credentials(
            "bucket",
            "account",
            "key",
            "secret",
            "https://endpoint.com"
        ).await.unwrap();
        
        // Add sync folder
        let folder_id = db.add_sync_folder(
            "/home/user/documents",
            "documents/"
        ).await.unwrap();
        
        assert!(folder_id > 0);
        
        // Get sync folders
        let folders = db.get_sync_folders().await.unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].local_path, "/home/user/documents");
        assert_eq!(folders[0].remote_path, "documents/");
        assert!(folders[0].enabled);
        
        // Toggle disabled
        db.toggle_sync_folder(folder_id, false).await.unwrap();
        let folders = db.get_sync_folders().await.unwrap();
        assert!(!folders[0].enabled);
        
        // Remove folder
        db.remove_sync_folder(folder_id).await.unwrap();
        let folders = db.get_sync_folders().await.unwrap();
        assert!(folders.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_sync_folders() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // Add bucket
        db.save_credentials(
            "bucket",
            "account",
            "key",
            "secret",
            "https://endpoint.com"
        ).await.unwrap();
        
        // Add multiple folders
        db.add_sync_folder("/path/to/docs", "docs/").await.unwrap();
        db.add_sync_folder("/path/to/photos", "photos/").await.unwrap();
        db.add_sync_folder("/path/to/videos", "videos/").await.unwrap();
        
        let folders = db.get_sync_folders().await.unwrap();
        assert_eq!(folders.len(), 3);
    }

    #[tokio::test]
    async fn test_sync_folders_without_bucket() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // No bucket added - get_sync_folders should return empty
        let folders = db.get_sync_folders().await.unwrap();
        assert!(folders.is_empty());
    }

    #[tokio::test]
    async fn test_empty_database() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // All queries should work on empty database
        let creds = db.load_credentials().await.unwrap();
        assert!(creds.is_none());
        
        let bucket = db.get_current_bucket().await.unwrap();
        assert!(bucket.is_none());
        
        let folders = db.get_sync_folders().await.unwrap();
        assert!(folders.is_empty());
    }

    #[tokio::test]
    async fn test_credentials_are_encrypted() {
        let (db, _temp_dir) = setup_test_db().await;
        
        // Save credentials with sensitive data
        let secret = "super-secret-key-12345";
        db.save_credentials(
            "bucket",
            "account",
            "access",
            secret,
            "https://endpoint.com"
        ).await.unwrap();
        
        // Check that stored data is encrypted (not plaintext)
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT access_key_id, secret_access_key FROM buckets LIMIT 1"
        )
        .fetch_optional(&db.pool)
        .await
        .unwrap();
        
        let (stored_access, stored_secret) = row.unwrap();
        
        // Stored values should NOT be plaintext
        assert_ne!(stored_access, "access");
        assert_ne!(stored_secret, secret);
        
        // But decrypted values should match original
        let loaded = db.load_credentials().await.unwrap().unwrap();
        assert_eq!(loaded.2, "access");
        assert_eq!(loaded.3, secret);
    }
}
