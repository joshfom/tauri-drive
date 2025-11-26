use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use anyhow::Result;
use std::path::PathBuf;

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
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

        Ok(Self { pool })
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
        .bind(access_key_id)
        .bind(secret_access_key)
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

        Ok(result)
    }

    pub async fn get_current_bucket(&self) -> Result<Option<String>> {
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM buckets ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(name,)| name))
    }
}
