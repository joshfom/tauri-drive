use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use anyhow::Result;
use crate::utils::{UploadProgress, UploadStatus};

pub struct UploadManager {
    pool: SqlitePool,
}

impl UploadManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_upload(
        &self,
        bucket_id: i64,
        file_path: &str,
        remote_path: &str,
        total_size: i64,
        chunk_size: i64,
    ) -> Result<String> {
        let upload_id = Uuid::new_v4().to_string();
        // Normalize path separators for cross-platform compatibility (Windows uses \)
        let normalized_path = file_path.replace('\\', "/");
        let _file_name = std::path::Path::new(&normalized_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        sqlx::query(
            r#"
            INSERT INTO uploads (id, bucket_id, file_path, remote_path, total_size, chunk_size, status, started_at)
            VALUES (?, ?, ?, ?, ?, ?, 'pending', datetime('now'))
            "#
        )
        .bind(&upload_id)
        .bind(bucket_id)
        .bind(file_path)
        .bind(remote_path)
        .bind(total_size)
        .bind(chunk_size)
        .execute(&self.pool)
        .await?;

        Ok(upload_id)
    }

    pub async fn update_upload_status(
        &self,
        upload_id: &str,
        status: &str,
        uploaded_size: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let mut query = String::from("UPDATE uploads SET status = ?");
        
        if uploaded_size.is_some() {
            query.push_str(", uploaded_size = ?");
        }
        if error_message.is_some() {
            query.push_str(", error_message = ?");
        }
        if status == "completed" || status == "failed" {
            query.push_str(", completed_at = datetime('now')");
        }
        query.push_str(" WHERE id = ?");

        let mut q = sqlx::query(&query).bind(status);
        
        if let Some(size) = uploaded_size {
            q = q.bind(size);
        }
        if let Some(err) = error_message {
            q = q.bind(err);
        }
        q = q.bind(upload_id);

        q.execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_multipart_upload_id(&self, upload_id: &str, multipart_id: &str) -> Result<()> {
        sqlx::query("UPDATE uploads SET upload_id = ? WHERE id = ?")
            .bind(multipart_id)
            .bind(upload_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_chunk(
        &self,
        upload_id: &str,
        part_number: i32,
        size: i64,
        etag: Option<&str>,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO upload_chunks (upload_id, part_number, size, etag, status, uploaded_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(upload_id, part_number) DO UPDATE SET
                etag = excluded.etag,
                status = excluded.status,
                uploaded_at = excluded.uploaded_at
            "#
        )
        .bind(upload_id)
        .bind(part_number)
        .bind(size)
        .bind(etag)
        .bind(status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_upload(&self, upload_id: &str) -> Result<Option<UploadProgress>> {
        let row = sqlx::query(
            r#"
            SELECT id, file_path, remote_path, total_size, uploaded_size, status, error_message
            FROM uploads WHERE id = ?
            "#
        )
        .bind(upload_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let file_path: String = row.try_get("file_path")?;
            // Normalize path separators for cross-platform compatibility (Windows uses \)
            let normalized_path = file_path.replace('\\', "/");
            let file_name = std::path::Path::new(&normalized_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let total_size: i64 = row.try_get("total_size")?;
            let uploaded_size: i64 = row.try_get("uploaded_size")?;
            let progress = if total_size > 0 {
                (uploaded_size as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            let status_str: String = row.try_get("status")?;
            let status = match status_str.as_str() {
                "pending" => UploadStatus::Pending,
                "uploading" => UploadStatus::Uploading,
                "paused" => UploadStatus::Paused,
                "completed" => UploadStatus::Completed,
                "failed" => UploadStatus::Failed,
                "cancelled" => UploadStatus::Cancelled,
                _ => UploadStatus::Pending,
            };

            Ok(Some(UploadProgress {
                id: row.try_get("id")?,
                file_name,
                file_path,
                remote_path: row.try_get("remote_path")?,
                total_size,
                uploaded_size,
                progress,
                speed: 0.0,
                eta: 0,
                status,
                error_message: row.try_get("error_message").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_active_uploads(&self) -> Result<Vec<UploadProgress>> {
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, remote_path, total_size, uploaded_size, status, error_message
            FROM uploads 
            WHERE status IN ('pending', 'uploading', 'paused')
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut uploads = Vec::new();
        for row in rows {
            let file_path: String = row.try_get("file_path")?;
            // Normalize path separators for cross-platform compatibility (Windows uses \)
            let normalized_path = file_path.replace('\\', "/");
            let file_name = std::path::Path::new(&normalized_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let total_size: i64 = row.try_get("total_size")?;
            let uploaded_size: i64 = row.try_get("uploaded_size")?;
            let progress = if total_size > 0 {
                (uploaded_size as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            let status_str: String = row.try_get("status")?;
            let status = match status_str.as_str() {
                "pending" => UploadStatus::Pending,
                "uploading" => UploadStatus::Uploading,
                "paused" => UploadStatus::Paused,
                "completed" => UploadStatus::Completed,
                "failed" => UploadStatus::Failed,
                "cancelled" => UploadStatus::Cancelled,
                _ => UploadStatus::Pending,
            };

            uploads.push(UploadProgress {
                id: row.try_get("id")?,
                file_name,
                file_path,
                remote_path: row.try_get("remote_path")?,
                total_size,
                uploaded_size,
                progress,
                speed: 0.0,
                eta: 0,
                status,
                error_message: row.try_get("error_message").ok(),
            });
        }

        Ok(uploads)
    }

    pub async fn get_completed_chunks(&self, upload_id: &str) -> Result<Vec<(i32, String)>> {
        let rows = sqlx::query(
            "SELECT part_number, etag FROM upload_chunks WHERE upload_id = ? AND status = 'completed' ORDER BY part_number"
        )
        .bind(upload_id)
        .fetch_all(&self.pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let part_number: i32 = row.try_get("part_number")?;
            let etag: String = row.try_get("etag")?;
            chunks.push((part_number, etag));
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&db_url).await.unwrap();
        
        // Run migrations
        let migrations = include_str!("../../migrations/001_init.sql");
        sqlx::query(migrations).execute(&pool).await.unwrap();
        
        // Add a test bucket first (required for foreign key)
        sqlx::query(
            "INSERT INTO buckets (name, account_id, access_key_id, secret_access_key, endpoint) 
             VALUES ('test-bucket', 'account', 'key', 'secret', 'https://endpoint.com')"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        (pool, temp_dir)
    }

    #[tokio::test]
    async fn test_create_upload() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/path/to/file.txt",
            "remote/file.txt",
            1024,
            256,
        ).await.unwrap();
        
        // UUID v4 should be 36 characters
        assert_eq!(upload_id.len(), 36);
    }

    #[tokio::test]
    async fn test_get_upload() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/path/to/document.pdf",
            "documents/document.pdf",
            2048,
            512,
        ).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap();
        assert!(upload.is_some());
        
        let upload = upload.unwrap();
        assert_eq!(upload.id, upload_id);
        assert_eq!(upload.file_name, "document.pdf");
        assert_eq!(upload.file_path, "/path/to/document.pdf");
        assert_eq!(upload.remote_path, "documents/document.pdf");
        assert_eq!(upload.total_size, 2048);
        assert_eq!(upload.uploaded_size, 0);
        assert!(matches!(upload.status, UploadStatus::Pending));
    }

    #[tokio::test]
    async fn test_get_nonexistent_upload() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload = manager.get_upload("nonexistent-id").await.unwrap();
        assert!(upload.is_none());
    }

    #[tokio::test]
    async fn test_update_upload_status() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/path/to/file.txt",
            "remote/file.txt",
            1024,
            256,
        ).await.unwrap();
        
        // Update to uploading
        manager.update_upload_status(&upload_id, "uploading", Some(512), None).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert!(matches!(upload.status, UploadStatus::Uploading));
        assert_eq!(upload.uploaded_size, 512);
        assert!((upload.progress - 50.0).abs() < 0.01);
        
        // Update to completed
        manager.update_upload_status(&upload_id, "completed", Some(1024), None).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert!(matches!(upload.status, UploadStatus::Completed));
        assert_eq!(upload.uploaded_size, 1024);
    }

    #[tokio::test]
    async fn test_update_upload_with_error() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/path/to/file.txt",
            "remote/file.txt",
            1024,
            256,
        ).await.unwrap();
        
        // Update to failed with error message
        manager.update_upload_status(
            &upload_id, 
            "failed", 
            None, 
            Some("Network error")
        ).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert!(matches!(upload.status, UploadStatus::Failed));
        assert_eq!(upload.error_message, Some("Network error".to_string()));
    }

    #[tokio::test]
    async fn test_get_active_uploads() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        // Create multiple uploads with different statuses
        let id1 = manager.create_upload(1, "/file1.txt", "file1.txt", 100, 50).await.unwrap();
        let id2 = manager.create_upload(1, "/file2.txt", "file2.txt", 200, 50).await.unwrap();
        let id3 = manager.create_upload(1, "/file3.txt", "file3.txt", 300, 50).await.unwrap();
        let id4 = manager.create_upload(1, "/file4.txt", "file4.txt", 400, 50).await.unwrap();
        
        manager.update_upload_status(&id1, "uploading", None, None).await.unwrap();
        manager.update_upload_status(&id2, "paused", None, None).await.unwrap();
        manager.update_upload_status(&id3, "completed", None, None).await.unwrap();
        manager.update_upload_status(&id4, "failed", None, Some("Error")).await.unwrap();
        
        // Get active uploads (pending, uploading, paused)
        let active = manager.get_active_uploads().await.unwrap();
        
        // id1 (uploading), id2 (paused) should be active
        // id3 (completed), id4 (failed) should not be active
        assert_eq!(active.len(), 2);
        assert!(active.iter().any(|u| u.id == id1));
        assert!(active.iter().any(|u| u.id == id2));
    }

    #[tokio::test]
    async fn test_save_and_get_chunks() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/path/to/large_file.zip",
            "large_file.zip",
            10 * 1024 * 1024, // 10MB
            5 * 1024 * 1024,  // 5MB chunks
        ).await.unwrap();
        
        // Save chunks
        manager.save_chunk(&upload_id, 1, 5 * 1024 * 1024, Some("etag1"), "completed").await.unwrap();
        manager.save_chunk(&upload_id, 2, 5 * 1024 * 1024, Some("etag2"), "completed").await.unwrap();
        
        // Get completed chunks
        let chunks = manager.get_completed_chunks(&upload_id).await.unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], (1, "etag1".to_string()));
        assert_eq!(chunks[1], (2, "etag2".to_string()));
    }

    #[tokio::test]
    async fn test_chunk_upsert() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/file.bin",
            "file.bin",
            1024,
            512,
        ).await.unwrap();
        
        // Save chunk first time
        manager.save_chunk(&upload_id, 1, 512, None, "uploading").await.unwrap();
        
        // Update same chunk (upsert)
        manager.save_chunk(&upload_id, 1, 512, Some("final_etag"), "completed").await.unwrap();
        
        // Should have only one chunk with updated values
        let chunks = manager.get_completed_chunks(&upload_id).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1, "final_etag");
    }

    #[tokio::test]
    async fn test_set_multipart_upload_id() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/large_file.bin",
            "large_file.bin",
            100 * 1024 * 1024,
            10 * 1024 * 1024,
        ).await.unwrap();
        
        // Set multipart upload ID (from S3)
        manager.set_multipart_upload_id(&upload_id, "aws-multipart-id-12345").await.unwrap();
        
        // Verify it was saved
        let row: (Option<String>,) = sqlx::query_as(
            "SELECT upload_id FROM uploads WHERE id = ?"
        )
        .bind(&upload_id)
        .fetch_one(manager.pool())
        .await
        .unwrap();
        
        assert_eq!(row.0, Some("aws-multipart-id-12345".to_string()));
    }

    #[tokio::test]
    async fn test_windows_path_normalization() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        // Test with Windows-style path
        let upload_id = manager.create_upload(
            1,
            r"C:\Users\test\Documents\file.txt",
            "documents/file.txt",
            1024,
            256,
        ).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        
        // File name should be extracted correctly even from Windows paths
        assert_eq!(upload.file_name, "file.txt");
    }

    #[tokio::test]
    async fn test_progress_calculation() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/file.bin",
            "file.bin",
            1000,
            100,
        ).await.unwrap();
        
        // 0% progress initially
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert_eq!(upload.progress, 0.0);
        
        // 25% progress
        manager.update_upload_status(&upload_id, "uploading", Some(250), None).await.unwrap();
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert!((upload.progress - 25.0).abs() < 0.01);
        
        // 100% progress
        manager.update_upload_status(&upload_id, "completed", Some(1000), None).await.unwrap();
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        assert!((upload.progress - 100.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_zero_size_file() {
        let (pool, _temp_dir) = setup_test_db().await;
        let manager = UploadManager::new(pool);
        
        let upload_id = manager.create_upload(
            1,
            "/empty.txt",
            "empty.txt",
            0,  // Zero size file
            256,
        ).await.unwrap();
        
        let upload = manager.get_upload(&upload_id).await.unwrap().unwrap();
        // Progress should be 0 (not NaN or panic)
        assert_eq!(upload.progress, 0.0);
    }

    // Add helper method for tests
    impl UploadManager {
        #[cfg(test)]
        fn pool(&self) -> &SqlitePool {
            &self.pool
        }
    }
}

