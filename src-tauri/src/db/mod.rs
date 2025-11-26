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
