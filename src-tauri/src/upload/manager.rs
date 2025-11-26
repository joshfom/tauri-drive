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
        let _file_name = std::path::Path::new(file_path)
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
            let file_name = std::path::Path::new(&file_path)
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
            let file_name = std::path::Path::new(&file_path)
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
