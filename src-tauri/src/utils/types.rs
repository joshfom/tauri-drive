use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Object {
    pub key: String,
    pub size: i64,
    pub last_modified: DateTime<Utc>,
    pub etag: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Bucket {
    pub name: String,
    pub region: Option<String>,
    pub creation_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub id: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "remotePath")]
    pub remote_path: String,
    #[serde(rename = "totalSize")]
    pub total_size: i64,
    #[serde(rename = "uploadedSize")]
    pub uploaded_size: i64,
    pub progress: f64,
    pub speed: f64,
    pub eta: i64,
    pub status: UploadStatus,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadStatus {
    Pending,
    Uploading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Credentials {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub chunk_size: usize,
    pub parallel_uploads: usize,
    pub bandwidth_limit: usize,
    pub conflict_resolution: String,
    pub notifications: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            chunk_size: 10 * 1024 * 1024, // 10MB
            parallel_uploads: 6,
            bandwidth_limit: 0,
            conflict_resolution: "ask".to_string(),
            notifications: true,
        }
    }
}
