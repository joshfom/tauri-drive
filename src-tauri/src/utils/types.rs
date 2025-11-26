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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_r2_object_serialization() {
        let obj = R2Object {
            key: "test/file.txt".to_string(),
            size: 1024,
            last_modified: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
            etag: "abc123".to_string(),
            is_directory: false,
        };

        let json = serde_json::to_string(&obj).unwrap();
        assert!(json.contains("test/file.txt"));
        assert!(json.contains("1024"));
        
        let deserialized: R2Object = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, obj.key);
        assert_eq!(deserialized.size, obj.size);
    }

    #[test]
    fn test_r2_object_directory() {
        let dir = R2Object {
            key: "test/folder/".to_string(),
            size: 0,
            last_modified: Utc::now(),
            etag: "".to_string(),
            is_directory: true,
        };

        assert!(dir.is_directory);
        assert!(dir.key.ends_with('/'));
    }

    #[test]
    fn test_upload_progress_serialization() {
        let progress = UploadProgress {
            id: "upload-123".to_string(),
            file_name: "document.pdf".to_string(),
            file_path: "/home/user/document.pdf".to_string(),
            remote_path: "uploads/document.pdf".to_string(),
            total_size: 1024 * 1024,
            uploaded_size: 512 * 1024,
            progress: 50.0,
            speed: 102400.0,
            eta: 5,
            status: UploadStatus::Uploading,
            error_message: None,
        };

        let json = serde_json::to_string(&progress).unwrap();
        
        // Check camelCase serialization
        assert!(json.contains("\"fileName\""));
        assert!(json.contains("\"filePath\""));
        assert!(json.contains("\"remotePath\""));
        assert!(json.contains("\"totalSize\""));
        assert!(json.contains("\"uploadedSize\""));
        
        let deserialized: UploadProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, progress.id);
        assert_eq!(deserialized.progress, 50.0);
    }

    #[test]
    fn test_upload_status_serialization() {
        let statuses = vec![
            (UploadStatus::Pending, "\"pending\""),
            (UploadStatus::Uploading, "\"uploading\""),
            (UploadStatus::Paused, "\"paused\""),
            (UploadStatus::Completed, "\"completed\""),
            (UploadStatus::Failed, "\"failed\""),
            (UploadStatus::Cancelled, "\"cancelled\""),
        ];

        for (status, expected) in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_upload_status_deserialization() {
        // Test pending
        let status: UploadStatus = serde_json::from_str("\"pending\"").unwrap();
        assert!(matches!(status, UploadStatus::Pending));
        
        // Test uploading
        let status: UploadStatus = serde_json::from_str("\"uploading\"").unwrap();
        assert!(matches!(status, UploadStatus::Uploading));
        
        // Test paused
        let status: UploadStatus = serde_json::from_str("\"paused\"").unwrap();
        assert!(matches!(status, UploadStatus::Paused));
        
        // Test completed
        let status: UploadStatus = serde_json::from_str("\"completed\"").unwrap();
        assert!(matches!(status, UploadStatus::Completed));
        
        // Test failed
        let status: UploadStatus = serde_json::from_str("\"failed\"").unwrap();
        assert!(matches!(status, UploadStatus::Failed));
        
        // Test cancelled
        let status: UploadStatus = serde_json::from_str("\"cancelled\"").unwrap();
        assert!(matches!(status, UploadStatus::Cancelled));
    }

    #[test]
    fn test_r2_credentials_serialization() {
        let creds = R2Credentials {
            account_id: "acc123".to_string(),
            access_key_id: "key456".to_string(),
            secret_access_key: "secret789".to_string(),
            endpoint: "https://example.r2.cloudflarestorage.com".to_string(),
        };

        let json = serde_json::to_string(&creds).unwrap();
        
        // Check camelCase serialization
        assert!(json.contains("\"accountId\""));
        assert!(json.contains("\"accessKeyId\""));
        assert!(json.contains("\"secretAccessKey\""));
        
        let deserialized: R2Credentials = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.account_id, creds.account_id);
        assert_eq!(deserialized.access_key_id, creds.access_key_id);
    }

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        
        assert_eq!(settings.theme, "system");
        assert_eq!(settings.chunk_size, 10 * 1024 * 1024);
        assert_eq!(settings.parallel_uploads, 6);
        assert_eq!(settings.bandwidth_limit, 0);
        assert_eq!(settings.conflict_resolution, "ask");
        assert!(settings.notifications);
    }

    #[test]
    fn test_app_settings_serialization() {
        let settings = AppSettings {
            theme: "dark".to_string(),
            chunk_size: 5 * 1024 * 1024,
            parallel_uploads: 4,
            bandwidth_limit: 1024 * 1024,
            conflict_resolution: "local".to_string(),
            notifications: false,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.theme, settings.theme);
        assert_eq!(deserialized.chunk_size, settings.chunk_size);
        assert_eq!(deserialized.parallel_uploads, settings.parallel_uploads);
        assert!(!deserialized.notifications);
    }

    #[test]
    fn test_upload_progress_with_error() {
        let progress = UploadProgress {
            id: "upload-456".to_string(),
            file_name: "failed.zip".to_string(),
            file_path: "/home/user/failed.zip".to_string(),
            remote_path: "uploads/failed.zip".to_string(),
            total_size: 1024 * 1024,
            uploaded_size: 0,
            progress: 0.0,
            speed: 0.0,
            eta: 0,
            status: UploadStatus::Failed,
            error_message: Some("Connection timeout".to_string()),
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"errorMessage\""));
        assert!(json.contains("Connection timeout"));
        
        let deserialized: UploadProgress = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.status, UploadStatus::Failed));
        assert_eq!(deserialized.error_message, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_r2_bucket_optional_fields() {
        let bucket_minimal = R2Bucket {
            name: "my-bucket".to_string(),
            region: None,
            creation_date: None,
        };

        let json = serde_json::to_string(&bucket_minimal).unwrap();
        let deserialized: R2Bucket = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "my-bucket");
        assert!(deserialized.region.is_none());
        assert!(deserialized.creation_date.is_none());
    }

    #[test]
    fn test_r2_bucket_with_all_fields() {
        let bucket = R2Bucket {
            name: "full-bucket".to_string(),
            region: Some("auto".to_string()),
            creation_date: Some(Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap()),
        };

        let json = serde_json::to_string(&bucket).unwrap();
        let deserialized: R2Bucket = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.name, "full-bucket");
        assert_eq!(deserialized.region, Some("auto".to_string()));
        assert!(deserialized.creation_date.is_some());
    }
}
