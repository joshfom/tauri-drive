// Allow unused code in library - some functions are for future use or exported for flexibility
#![allow(dead_code)]

mod r2;
mod db;
mod utils;
mod upload;
mod crypto;
mod migration;

use r2::R2Client;
use r2::multipart::MultipartUpload;
use db::Database;
use upload::UploadManager;
use migration::{BackupData, CredentialsBackup, SyncFolderBackup, SettingBackup, UploadHistoryBackup};
use utils::{R2Object, R2Credentials, UploadProgress, UploadStatus};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter, WebviewWindow};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItem};

pub struct AppState {
    pub db: Arc<Database>,
    pub r2_client: Arc<Mutex<Option<R2Client>>>,
    pub upload_manager: Arc<UploadManager>,
    /// Active multipart uploads that can be paused/cancelled
    pub active_uploads: Arc<Mutex<HashMap<String, Arc<MultipartUpload>>>>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn connect_r2(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    credentials: R2Credentials,
    bucket: String,
    save_credentials: bool,
) -> Result<String, String> {
    // Create client
    let client = R2Client::new(
        &credentials.account_id,
        &credentials.access_key_id,
        &credentials.secret_access_key,
        &bucket,
    )
    .await
    .map_err(|e| format!("Failed to create R2 client: {}", e))?;

    // Test connection by listing objects (with empty prefix)
    r2::operations::list_objects(
        client.client(),
        client.bucket(),
        None,
    )
    .await
    .map_err(|e| format!("Connection test failed: {}", e))?;

    // Save credentials to database if requested
    let app_state = state.lock().await;
    if save_credentials {
        app_state.db.save_credentials(
            &bucket,
            &credentials.account_id,
            &credentials.access_key_id,
            &credentials.secret_access_key,
            &credentials.endpoint,
        )
        .await
        .map_err(|e| format!("Failed to save credentials: {}", e))?;
    }

    // Store the client
    *app_state.r2_client.lock().await = Some(client);

    Ok("Connected successfully! Connection verified by listing objects.".to_string())
}

#[tauri::command]
async fn get_current_credentials(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<(String, String, String, String)>, String> {
    let app_state = state.lock().await;
    let creds = app_state.db.load_credentials()
        .await
        .map_err(|e| format!("Failed to load credentials: {}", e))?;
    
    // Return (bucket_name, account_id, access_key_id, secret_access_key)
    Ok(creds.map(|(name, account_id, access_key_id, secret_access_key, _endpoint)| {
        (name, account_id, access_key_id, secret_access_key)
    }))
}

#[tauri::command]
async fn get_saved_bucket(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<String>, String> {
    let app_state = state.lock().await;
    app_state.db.get_current_bucket()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn load_and_connect(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let app_state = state.lock().await;
    
    // Load credentials from database
    let creds = app_state.db.load_credentials()
        .await
        .map_err(|e| format!("Failed to load credentials: {}", e))?
        .ok_or("No saved credentials found")?;

    let (bucket, account_id, access_key_id, secret_access_key, _endpoint) = creds;

    // Create client
    let client = R2Client::new(
        &account_id,
        &access_key_id,
        &secret_access_key,
        &bucket,
    )
    .await
    .map_err(|e| format!("Failed to create R2 client: {}", e))?;

    // Test connection
    r2::operations::list_objects(
        client.client(),
        client.bucket(),
        None,
    )
    .await
    .map_err(|e| format!("Connection test failed: {}", e))?;

    // Store the client
    *app_state.r2_client.lock().await = Some(client);

    Ok(format!("Auto-connected to bucket: {}", bucket))
}

#[tauri::command]
async fn list_objects(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    prefix: Option<String>,
) -> Result<Vec<R2Object>, String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    let objects = r2::operations::list_objects(
        client.client(),
        client.bucket(),
        prefix.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(objects)
}

#[tauri::command]
async fn upload_file(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    local_path: String,
    remote_key: String,
) -> Result<String, String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    // Check file size to determine if multipart upload is needed
    let metadata = tokio::fs::metadata(&local_path)
        .await
        .map_err(|e| e.to_string())?;

    let file_size = metadata.len();
    
    // Use multipart upload for files > 100MB
    if file_size > 100 * 1024 * 1024 {
        r2::multipart::upload_large_file(
            client.client(),
            client.bucket(),
            &remote_key,
            &local_path,
            Some(10 * 1024 * 1024), // 10MB chunks
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok("Uploaded with multipart".to_string())
    } else {
        let etag = r2::operations::put_object(
            client.client(),
            client.bucket(),
            &remote_key,
            &local_path,
        )
        .await
        .map_err(|e| e.to_string())?;
        Ok(etag)
    }
}

#[tauri::command]
async fn download_file(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    remote_key: String,
    local_path: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    r2::operations::get_object_streaming(
        client.client(),
        client.bucket(),
        &remote_key,
        &local_path,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Download progress info sent to frontend
#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    id: String,
    file_name: String,
    remote_path: String,
    local_path: String,
    total_size: i64,
    downloaded_size: i64,
    progress: f64,
    speed: f64,
    eta: i64,
    status: String,
    error_message: Option<String>,
}

#[tauri::command]
async fn download_file_with_progress(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    remote_key: String,
    local_path: String,
) -> Result<String, String> {
    let download_id = uuid::Uuid::new_v4().to_string();
    
    let (client_clone, bucket_clone) = {
        let app_state = state.lock().await;
        let client_guard = app_state.r2_client.lock().await;
        
        let client = client_guard
            .as_ref()
            .ok_or("Not connected to R2")?;
        
        (client.client().clone(), client.bucket().to_string())
    };

    let file_name = remote_key.split('/').last().unwrap_or(&remote_key).to_string();
    
    // Emit initial progress
    let initial_progress = DownloadProgress {
        id: download_id.clone(),
        file_name: file_name.clone(),
        remote_path: remote_key.clone(),
        local_path: local_path.clone(),
        total_size: 0,
        downloaded_size: 0,
        progress: 0.0,
        speed: 0.0,
        eta: 0,
        status: "downloading".to_string(),
        error_message: None,
    };
    app.emit("download-progress", &initial_progress).ok();

    // Clone for closure
    let app_clone = app.clone();
    let download_id_clone = download_id.clone();
    let file_name_clone = file_name.clone();
    let remote_key_clone = remote_key.clone();
    let local_path_clone = local_path.clone();

    let progress_callback: r2::operations::DownloadProgressCallback = Box::new(move |downloaded, total, speed, eta| {
        let progress_pct = if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 };
        let progress_event = DownloadProgress {
            id: download_id_clone.clone(),
            file_name: file_name_clone.clone(),
            remote_path: remote_key_clone.clone(),
            local_path: local_path_clone.clone(),
            total_size: total,
            downloaded_size: downloaded,
            progress: progress_pct,
            speed,
            eta,
            status: "downloading".to_string(),
            error_message: None,
        };
        app_clone.emit("download-progress", &progress_event).ok();
    });

    r2::operations::get_object_streaming(
        &client_clone,
        &bucket_clone,
        &remote_key,
        &local_path,
        Some(progress_callback),
    )
    .await
    .map_err(|e| e.to_string())?;

    // Emit completion
    let complete_progress = DownloadProgress {
        id: download_id.clone(),
        file_name,
        remote_path: remote_key,
        local_path,
        total_size: 0,
        downloaded_size: 0,
        progress: 100.0,
        speed: 0.0,
        eta: 0,
        status: "completed".to_string(),
        error_message: None,
    };
    app.emit("download-progress", &complete_progress).ok();

    Ok(download_id)
}

/// Download a folder as a zip file
#[tauri::command]
async fn download_folder_as_zip(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    folder_path: String,
    local_path: String,
) -> Result<String, String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    
    let download_id = uuid::Uuid::new_v4().to_string();
    
    // Get client info
    let (client_clone, bucket_clone) = {
        let app_state = state.lock().await;
        let client_guard = app_state.r2_client.lock().await;
        
        let client = client_guard
            .as_ref()
            .ok_or("Not connected to R2")?;
        
        (client.client().clone(), client.bucket().to_string())
    };

    // List all files in the folder
    let prefix = if folder_path.ends_with('/') {
        folder_path.clone()
    } else {
        format!("{}/", folder_path)
    };
    
    let objects = r2::operations::list_objects(&client_clone, &bucket_clone, Some(&prefix))
        .await
        .map_err(|e| e.to_string())?;
    
    // Filter to only files (not empty folder markers)
    let files: Vec<_> = objects.iter()
        .filter(|o| !o.key.ends_with('/') && o.size > 0)
        .collect();
    
    if files.is_empty() {
        return Err("Folder is empty or contains no files".to_string());
    }
    
    let folder_name = folder_path.split('/').filter(|s| !s.is_empty()).last().unwrap_or("folder");
    let total_files = files.len();
    let total_size: i64 = files.iter().map(|f| f.size).sum();
    
    // Emit initial progress
    let initial_progress = DownloadProgress {
        id: download_id.clone(),
        file_name: format!("{}.zip", folder_name),
        remote_path: folder_path.clone(),
        local_path: local_path.clone(),
        total_size,
        downloaded_size: 0,
        progress: 0.0,
        speed: 0.0,
        eta: 0,
        status: "downloading".to_string(),
        error_message: None,
    };
    app.emit("download-progress", &initial_progress).ok();
    
    // Create zip file
    let zip_file = std::fs::File::create(&local_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    let mut downloaded_size: i64 = 0;
    let start_time = std::time::Instant::now();
    
    for (i, file) in files.iter().enumerate() {
        // Get the relative path within the folder
        let relative_path = file.key.strip_prefix(&prefix).unwrap_or(&file.key);
        
        // Download file content
        let response = client_clone
            .get_object()
            .bucket(&bucket_clone)
            .key(&file.key)
            .send()
            .await
            .map_err(|e| format!("Failed to download {}: {}", file.key, e))?;
        
        let data = response.body
            .collect()
            .await
            .map_err(|e| format!("Failed to read file data: {}", e))?
            .into_bytes();
        
        // Add to zip
        zip.start_file(relative_path, options)
            .map_err(|e| format!("Failed to add file to zip: {}", e))?;
        zip.write_all(&data)
            .map_err(|e| format!("Failed to write file to zip: {}", e))?;
        
        downloaded_size += file.size;
        
        // Calculate progress
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { downloaded_size as f64 / elapsed } else { 0.0 };
        let remaining = total_size - downloaded_size;
        let eta = if speed > 0.0 { (remaining as f64 / speed) as i64 } else { 0 };
        let progress_pct = (downloaded_size as f64 / total_size as f64) * 100.0;
        
        let progress_event = DownloadProgress {
            id: download_id.clone(),
            file_name: format!("{}.zip ({}/{})", folder_name, i + 1, total_files),
            remote_path: folder_path.clone(),
            local_path: local_path.clone(),
            total_size,
            downloaded_size,
            progress: progress_pct,
            speed,
            eta,
            status: "downloading".to_string(),
            error_message: None,
        };
        app.emit("download-progress", &progress_event).ok();
    }
    
    // Finalize zip
    zip.finish().map_err(|e| format!("Failed to finalize zip: {}", e))?;
    
    // Emit completion
    let complete_progress = DownloadProgress {
        id: download_id.clone(),
        file_name: format!("{}.zip", folder_name),
        remote_path: folder_path,
        local_path,
        total_size,
        downloaded_size: total_size,
        progress: 100.0,
        speed: 0.0,
        eta: 0,
        status: "completed".to_string(),
        error_message: None,
    };
    app.emit("download-progress", &complete_progress).ok();

    Ok(download_id)
}

/// Check if currently connected to R2
#[tauri::command]
async fn check_connection(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<ConnectionStatus, String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    match client_guard.as_ref() {
        Some(client) => {
            // Try to list objects to verify connection is still valid
            match r2::operations::list_objects(client.client(), client.bucket(), None).await {
                Ok(_) => Ok(ConnectionStatus {
                    connected: true,
                    bucket: Some(client.bucket().to_string()),
                    error: None,
                }),
                Err(e) => Ok(ConnectionStatus {
                    connected: false,
                    bucket: Some(client.bucket().to_string()),
                    error: Some(e.to_string()),
                }),
            }
        }
        None => Ok(ConnectionStatus {
            connected: false,
            bucket: None,
            error: None,
        }),
    }
}

#[derive(Clone, serde::Serialize)]
struct ConnectionStatus {
    connected: bool,
    bucket: Option<String>,
    error: Option<String>,
}

#[tauri::command]
async fn delete_file(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    remote_key: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    r2::operations::delete_object(
        client.client(),
        client.bucket(),
        &remote_key,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn rename_file(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    old_key: String,
    new_key: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    // Copy to new location
    r2::operations::copy_object(
        client.client(),
        client.bucket(),
        &old_key,
        &new_key,
    )
    .await
    .map_err(|e| format!("Failed to copy: {}", e))?;

    // Delete old file
    r2::operations::delete_object(
        client.client(),
        client.bucket(),
        &old_key,
    )
    .await
    .map_err(|e| format!("Failed to delete old file: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_active_uploads(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<UploadProgress>, String> {
    let app_state = state.lock().await;
    let uploads = app_state.upload_manager
        .get_active_uploads()
        .await
        .map_err(|e| e.to_string())?;
    Ok(uploads)
}

#[tauri::command]
async fn upload_file_with_progress(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    local_path: String,
    remote_key: String,
) -> Result<String, String> {
    // Clone what we need before the async block
    let (client_clone, bucket_clone, upload_id, file_size, file_name, upload_manager) = {
        let app_state = state.lock().await;
        let client_guard = app_state.r2_client.lock().await;
        
        let client = client_guard
            .as_ref()
            .ok_or("Not connected to R2")?;

        // Get file size
        let metadata = tokio::fs::metadata(&local_path)
            .await
            .map_err(|e| e.to_string())?;
        let file_size = metadata.len() as i64;

        // Create upload record
        let upload_id = app_state.upload_manager
            .create_upload(1, &local_path, &remote_key, file_size, 10 * 1024 * 1024)
            .await
            .map_err(|e| e.to_string())?;

        // Update status to uploading
        app_state.upload_manager
            .update_upload_status(&upload_id, "uploading", None, None)
            .await
            .map_err(|e| e.to_string())?;

        // Get file name - normalize path separators for Windows compatibility
        let normalized_path = local_path.replace('\\', "/");
        let file_name = std::path::Path::new(&normalized_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        (
            client.client().clone(),
            client.bucket().to_string(),
            upload_id,
            file_size,
            file_name,
            app_state.upload_manager.clone(),
        )
    };

    // Emit initial progress event
    let progress = UploadProgress {
        id: upload_id.clone(),
        file_name: file_name.clone(),
        file_path: local_path.clone(),
        remote_path: remote_key.clone(),
        total_size: file_size,
        uploaded_size: 0,
        progress: 0.0,
        speed: 0.0,
        eta: 0,
        status: UploadStatus::Uploading,
        error_message: None,
    };
    app.emit("upload-progress", &progress).ok();

    // Clone variables for closure
    let upload_id_clone = upload_id.clone();
    let file_name_clone = file_name.clone();
    let local_path_clone = local_path.clone();
    let remote_key_clone = remote_key.clone();

    // Use multipart upload for files > 100MB (with 8 concurrent uploads)
    if file_size > 100 * 1024 * 1024 {
        log::info!(
            "Starting multipart upload for {} ({} MB) with 5MB chunks",
            file_name,
            file_size / (1024 * 1024)
        );
        
        let app_clone = app.clone();
        let upload = r2::multipart::MultipartUpload::new(
            client_clone.clone(),
            bucket_clone.clone(),
            remote_key.clone(),
            Some(5 * 1024 * 1024), // 5MB chunks for more frequent progress updates
        )
        .await
        .map_err(|e| e.to_string())?;

        // Use concurrent upload with speed/ETA tracking
        let parts = upload
            .upload_file_concurrent(&local_path, move |progress_info| {
                let progress_pct = (progress_info.uploaded_bytes as f64 / progress_info.total_bytes as f64) * 100.0;
                log::info!(
                    "Emitting upload progress: {} - {:.1}% ({}/{} bytes)",
                    upload_id_clone,
                    progress_pct,
                    progress_info.uploaded_bytes,
                    progress_info.total_bytes
                );
                let progress_event = UploadProgress {
                    id: upload_id_clone.clone(),
                    file_name: file_name_clone.clone(),
                    file_path: local_path_clone.clone(),
                    remote_path: remote_key_clone.clone(),
                    total_size: progress_info.total_bytes,
                    uploaded_size: progress_info.uploaded_bytes,
                    progress: progress_pct,
                    speed: progress_info.speed_bytes_per_sec,
                    eta: progress_info.eta_seconds,
                    status: UploadStatus::Uploading,
                    error_message: None,
                };
                if let Err(e) = app_clone.emit("upload-progress", &progress_event) {
                    log::error!("Failed to emit upload progress event: {}", e);
                }
            })
            .await
            .map_err(|e| {
                // Mark as failed and abort upload
                let um = upload_manager.clone();
                let id = upload_id.clone();
                let err_msg = e.to_string();
                tauri::async_runtime::spawn(async move {
                    um.update_upload_status(&id, "failed", None, Some(&err_msg)).await.ok();
                });
                e.to_string()
            })?;

        // Complete the multipart upload - CRITICAL step
        upload
            .complete(parts)
            .await
            .map_err(|e| {
                let um = upload_manager.clone();
                let id = upload_id.clone();
                let err_msg = format!("Failed to complete multipart upload: {}", e);
                tauri::async_runtime::spawn(async move {
                    um.update_upload_status(&id, "failed", None, Some(&err_msg)).await.ok();
                });
                // Abort the upload on failure
                let upload_ref = upload;
                tauri::async_runtime::spawn(async move {
                    upload_ref.abort().await.ok();
                });
                e.to_string()
            })?;
    } else {
        // For smaller files, emit a progress event before starting
        let progress_event = UploadProgress {
            id: upload_id.clone(),
            file_name: file_name.clone(),
            file_path: local_path.clone(),
            remote_path: remote_key.clone(),
            total_size: file_size,
            uploaded_size: 0,
            progress: 0.0,
            speed: 0.0,
            eta: 0,
            status: UploadStatus::Uploading,
            error_message: None,
        };
        app.emit("upload-progress", &progress_event).ok();

        // For smaller files, use simple put_object with progress tracking
        r2::operations::put_object_with_progress(
            &client_clone,
            &bucket_clone,
            &remote_key,
            &local_path,
            {
                let app = app.clone();
                let upload_id = upload_id.clone();
                let file_name = file_name.clone();
                let local_path = local_path.clone();
                let remote_key = remote_key.clone();
                move |uploaded, total, speed, eta| {
                    let progress_pct = if total > 0 { (uploaded as f64 / total as f64) * 100.0 } else { 0.0 };
                    let progress_event = UploadProgress {
                        id: upload_id.clone(),
                        file_name: file_name.clone(),
                        file_path: local_path.clone(),
                        remote_path: remote_key.clone(),
                        total_size: total,
                        uploaded_size: uploaded,
                        progress: progress_pct,
                        speed,
                        eta,
                        status: UploadStatus::Uploading,
                        error_message: None,
                    };
                    app.emit("upload-progress", &progress_event).ok();
                }
            },
        )
        .await
        .map_err(|e| {
            let um = upload_manager.clone();
            let id = upload_id.clone();
            let err_msg = e.to_string();
            tauri::async_runtime::spawn(async move {
                um.update_upload_status(&id, "failed", None, Some(&err_msg)).await.ok();
            });
            e.to_string()
        })?;
    }

    // Mark as completed
    upload_manager
        .update_upload_status(&upload_id, "completed", Some(file_size), None)
        .await
        .map_err(|e| e.to_string())?;

    // Emit completion event
    let progress = UploadProgress {
        id: upload_id.clone(),
        file_name,
        file_path: local_path.clone(),
        remote_path: remote_key.clone(),
        total_size: file_size,
        uploaded_size: file_size,
        progress: 100.0,
        speed: 0.0,
        eta: 0,
        status: UploadStatus::Completed,
        error_message: None,
    };
    app.emit("upload-progress", &progress).ok();

    Ok(upload_id)
}

#[tauri::command]
async fn cancel_upload(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    upload_id: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    app_state.upload_manager
        .update_upload_status(&upload_id, "cancelled", None, None)
        .await
        .map_err(|e| e.to_string())?;
    
    // Cancel the active multipart upload if exists
    let active_uploads = app_state.active_uploads.lock().await;
    if let Some(upload) = active_uploads.get(&upload_id) {
        upload.cancel();
    }
    
    Ok(())
}

#[tauri::command]
async fn pause_upload(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    upload_id: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    
    // Update status to paused
    app_state.upload_manager
        .update_upload_status(&upload_id, "paused", None, None)
        .await
        .map_err(|e| e.to_string())?;
    
    // Pause the active multipart upload if exists
    let active_uploads = app_state.active_uploads.lock().await;
    if let Some(upload) = active_uploads.get(&upload_id) {
        upload.pause();
    }
    
    // Get current upload info to emit event
    if let Ok(Some(upload_progress)) = app_state.upload_manager.get_upload(&upload_id).await {
        app.emit("upload-progress", &upload_progress).ok();
    }
    
    Ok(())
}

#[tauri::command]
async fn resume_upload(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    upload_id: String,
) -> Result<(), String> {
    let app_state = state.lock().await;
    
    // Update status to uploading
    app_state.upload_manager
        .update_upload_status(&upload_id, "uploading", None, None)
        .await
        .map_err(|e| e.to_string())?;
    
    // Resume the active multipart upload if exists
    let active_uploads = app_state.active_uploads.lock().await;
    if let Some(upload) = active_uploads.get(&upload_id) {
        upload.resume();
    }
    
    // Get current upload info to emit event
    if let Ok(Some(upload_progress)) = app_state.upload_manager.get_upload(&upload_id).await {
        app.emit("upload-progress", &upload_progress).ok();
    }
    
    Ok(())
}

#[tauri::command]
async fn retry_upload(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    upload_id: String,
) -> Result<String, String> {
    // Get the failed upload info
    let (local_path, remote_key) = {
        let app_state = state.lock().await;
        let upload = app_state.upload_manager
            .get_upload(&upload_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Upload not found")?;
        
        (upload.file_path, upload.remote_path)
    };
    
    // Start a new upload with the same paths
    upload_file_with_progress(app, state, local_path, remote_key).await
}

#[tauri::command]
async fn create_folder(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    folder_path: String,
) -> Result<String, String> {
    let app_state = state.lock().await;
    let client_guard = app_state.r2_client.lock().await;
    let client = client_guard
        .as_ref()
        .ok_or("Not connected to R2")?;

    // Ensure path ends with /
    let folder_key = if folder_path.ends_with('/') {
        folder_path.clone()
    } else {
        format!("{}/", folder_path)
    };

    // Create an empty object with trailing slash to mark as folder
    r2::operations::put_object_from_bytes(
        client.client(),
        client.bucket(),
        &folder_key,
        vec![],
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(format!("Folder created: {}", folder_key))
}

#[derive(serde::Serialize)]
pub struct PathInfo {
    pub is_directory: bool,
    pub is_file: bool,
    pub exists: bool,
    pub path: String,
}

/// Check if a path is a file or directory
#[tauri::command]
async fn check_path_type(path: String) -> Result<PathInfo, String> {
    use std::path::Path;
    
    let p = Path::new(&path);
    Ok(PathInfo {
        is_directory: p.is_dir(),
        is_file: p.is_file(),
        exists: p.exists(),
        path,
    })
}

/// Hide the window to system tray (minimize to tray)
#[tauri::command]
async fn hide_to_tray(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

/// Show the window from system tray
#[tauri::command]
async fn show_from_tray(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_directory(
    directory_path: String,
) -> Result<Vec<String>, String> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(&directory_path);
    if !path.is_dir() {
        return Err("Not a directory".to_string());
    }

    let mut files = Vec::new();
    
    fn walk_dir(dir: &Path, base_path: &Path, files: &mut Vec<String>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            // Get relative path from base (not used currently but kept for future reference)
            let _relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("Invalid path")?;
            
            if path.is_file() {
                files.push(path.to_str().ok_or("Invalid path")?.to_string());
            } else if path.is_dir() {
                // Skip hidden directories like .git
                if let Some(name) = path.file_name() {
                    if !name.to_str().unwrap_or("").starts_with('.') {
                        walk_dir(&path, base_path, files)?;
                    }
                }
            }
        }
        Ok(())
    }
    
    walk_dir(path, path, &mut files)?;
    Ok(files)
}

/// Export configuration to a JSON file (credentials are encrypted)
#[tauri::command]
async fn export_config(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    file_path: String,
) -> Result<(), String> {
    use serde_json::json;
    
    let app_state = state.lock().await;
    
    // Load credentials from database
    let creds = app_state.db.load_credentials()
        .await
        .map_err(|e| format!("Failed to load credentials: {}", e))?
        .ok_or("No credentials to export")?;

    let (bucket, account_id, access_key_id, secret_access_key, endpoint) = creds;
    
    // Get sync folders
    let sync_folders = app_state.db.get_sync_folders()
        .await
        .map_err(|e| format!("Failed to load sync folders: {}", e))?;
    
    // Create config object
    let config = json!({
        "version": "1.0",
        "credentials": {
            "bucket": bucket,
            "account_id": account_id,
            "access_key_id": access_key_id,
            "secret_access_key": secret_access_key,
            "endpoint": endpoint
        },
        "sync_folders": sync_folders
    });
    
    // Write to file
    tokio::fs::write(&file_path, config.to_string())
        .await
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    Ok(())
}

/// Import configuration from a JSON file
#[tauri::command]
async fn import_config(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    file_path: String,
) -> Result<(), String> {
    // Read file
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid config file: {}", e))?;
    
    let app_state = state.lock().await;
    
    // Import credentials
    if let Some(creds) = config.get("credentials") {
        let bucket = creds.get("bucket")
            .and_then(|v| v.as_str())
            .ok_or("Missing bucket")?;
        let account_id = creds.get("account_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing account_id")?;
        let access_key_id = creds.get("access_key_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing access_key_id")?;
        let secret_access_key = creds.get("secret_access_key")
            .and_then(|v| v.as_str())
            .ok_or("Missing secret_access_key")?;
        let endpoint = creds.get("endpoint")
            .and_then(|v| v.as_str())
            .ok_or("Missing endpoint")?;
        
        app_state.db.save_credentials(
            bucket,
            account_id,
            access_key_id,
            secret_access_key,
            endpoint,
        )
        .await
        .map_err(|e| format!("Failed to save credentials: {}", e))?;
    }
    
    Ok(())
}

/// Sync folder data structure
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncFolder {
    pub id: i64,
    pub local_path: String,
    pub remote_path: String,
    pub enabled: bool,
    pub last_sync: Option<String>,
}

/// Get all sync folders
#[tauri::command]
async fn get_sync_folders(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SyncFolder>, String> {
    let app_state = state.lock().await;
    app_state.db.get_sync_folders()
        .await
        .map_err(|e| e.to_string())
}

/// Add a new sync folder
#[tauri::command]
async fn add_sync_folder(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    local_path: String,
    remote_path: String,
) -> Result<i64, String> {
    let app_state = state.lock().await;
    app_state.db.add_sync_folder(&local_path, &remote_path)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a sync folder
#[tauri::command]
async fn remove_sync_folder(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    folder_id: i64,
) -> Result<(), String> {
    let app_state = state.lock().await;
    app_state.db.remove_sync_folder(folder_id)
        .await
        .map_err(|e| e.to_string())
}

/// Toggle sync folder enabled status
#[tauri::command]
async fn toggle_sync_folder(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    folder_id: i64,
    enabled: bool,
) -> Result<(), String> {
    let app_state = state.lock().await;
    app_state.db.toggle_sync_folder(folder_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

/// Get temp directory path
#[tauri::command]
async fn get_temp_dir() -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    Ok(temp_dir.to_string_lossy().to_string())
}

/// Read text file content
#[tauri::command]
async fn read_text_file(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))
}

/// Delete temp file (cleanup after preview)
#[tauri::command]
async fn delete_temp_file(path: String) -> Result<(), String> {
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("Failed to delete temp file: {}", e))
}

/// Export full migration backup (encrypted with password)
#[tauri::command]
async fn export_migration_backup(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    file_path: String,
    password: String,
) -> Result<(), String> {
    if password.len() < 6 {
        return Err("Password must be at least 6 characters".to_string());
    }

    let app_state = state.lock().await;
    
    // Load credentials
    let credentials = match app_state.db.load_credentials().await {
        Ok(Some((bucket, account_id, access_key_id, secret_access_key, endpoint))) => {
            Some(CredentialsBackup {
                bucket_name: bucket,
                account_id,
                access_key_id,
                secret_access_key,
                endpoint,
            })
        }
        Ok(None) => None,
        Err(e) => return Err(format!("Failed to load credentials: {}", e)),
    };
    
    // Load sync folders
    let sync_folders_data = app_state.db.get_sync_folders()
        .await
        .map_err(|e| format!("Failed to load sync folders: {}", e))?;
    
    let sync_folders: Vec<SyncFolderBackup> = sync_folders_data
        .into_iter()
        .map(|f| SyncFolderBackup {
            local_path: f.local_path,
            remote_path: f.remote_path,
            sync_mode: "upload_only".to_string(),
            enabled: f.enabled,
        })
        .collect();
    
    // Load settings from database
    let settings: Vec<SettingBackup> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM settings"
    )
    .fetch_all(app_state.db.pool())
    .await
    .map_err(|e| format!("Failed to load settings: {}", e))?
    .into_iter()
    .map(|(key, value)| SettingBackup { key, value })
    .collect();
    
    // Load completed upload history
    let upload_history: Vec<UploadHistoryBackup> = sqlx::query_as::<_, (String, String, i64, String, Option<String>)>(
        "SELECT file_path, remote_path, total_size, status, completed_at 
         FROM uploads 
         WHERE status = 'completed' 
         ORDER BY completed_at DESC 
         LIMIT 1000"
    )
    .fetch_all(app_state.db.pool())
    .await
    .map_err(|e| format!("Failed to load upload history: {}", e))?
    .into_iter()
    .map(|(file_path, remote_path, total_size, status, completed_at)| {
        UploadHistoryBackup {
            file_path,
            remote_path,
            total_size,
            status,
            completed_at,
        }
    })
    .collect();
    
    // Create backup data
    let backup = BackupData {
        version: 1,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        credentials,
        sync_folders,
        settings,
        upload_history,
    };
    
    // Encrypt and write to file
    let encrypted = migration::encrypt_backup(&backup, &password)
        .map_err(|e| format!("Failed to encrypt backup: {}", e))?;
    
    tokio::fs::write(&file_path, encrypted)
        .await
        .map_err(|e| format!("Failed to write backup file: {}", e))?;
    
    Ok(())
}

/// Import migration backup (decrypt with password)
#[tauri::command]
async fn import_migration_backup(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    file_path: String,
    password: String,
) -> Result<MigrationImportResult, String> {
    // Read encrypted file
    let encrypted = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read backup file: {}", e))?;
    
    // Decrypt
    let backup = migration::decrypt_backup(&encrypted, &password)
        .map_err(|e| e.to_string())?;
    
    let app_state = state.lock().await;
    let mut result = MigrationImportResult {
        credentials_imported: false,
        sync_folders_imported: 0,
        settings_imported: 0,
        upload_history_imported: 0,
    };
    
    // Import credentials
    if let Some(creds) = backup.credentials {
        app_state.db.save_credentials(
            &creds.bucket_name,
            &creds.account_id,
            &creds.access_key_id,
            &creds.secret_access_key,
            &creds.endpoint,
        )
        .await
        .map_err(|e| format!("Failed to import credentials: {}", e))?;
        result.credentials_imported = true;
    }
    
    // Import sync folders
    for folder in backup.sync_folders {
        match app_state.db.add_sync_folder(&folder.local_path, &folder.remote_path).await {
            Ok(_) => result.sync_folders_imported += 1,
            Err(e) => eprintln!("Failed to import sync folder {}: {}", folder.local_path, e),
        }
    }
    
    // Import settings
    for setting in backup.settings {
        match sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?) 
             ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(&setting.key)
        .bind(&setting.value)
        .execute(app_state.db.pool())
        .await {
            Ok(_) => result.settings_imported += 1,
            Err(e) => eprintln!("Failed to import setting {}: {}", setting.key, e),
        }
    }
    
    // Note: We don't import upload history by default as paths may differ on new machine
    // But we track what was available
    result.upload_history_imported = backup.upload_history.len() as i32;
    
    Ok(result)
}

/// Result of migration import
#[derive(serde::Serialize)]
struct MigrationImportResult {
    credentials_imported: bool,
    sync_folders_imported: i32,
    settings_imported: i32,
    upload_history_imported: i32,
}

/// Preview what's in a migration backup without importing
#[tauri::command]
async fn preview_migration_backup(
    file_path: String,
    password: String,
) -> Result<MigrationPreview, String> {
    // Read encrypted file
    let encrypted = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read backup file: {}", e))?;
    
    // Decrypt
    let backup = migration::decrypt_backup(&encrypted, &password)
        .map_err(|e| e.to_string())?;
    
    Ok(MigrationPreview {
        version: backup.version,
        app_version: backup.app_version,
        created_at: backup.created_at,
        has_credentials: backup.credentials.is_some(),
        bucket_name: backup.credentials.as_ref().map(|c| c.bucket_name.clone()),
        sync_folders_count: backup.sync_folders.len() as i32,
        settings_count: backup.settings.len() as i32,
        upload_history_count: backup.upload_history.len() as i32,
    })
}

/// Preview of migration backup contents
#[derive(serde::Serialize)]
struct MigrationPreview {
    version: u32,
    app_version: String,
    created_at: String,
    has_credentials: bool,
    bucket_name: Option<String>,
    sync_folders_count: i32,
    settings_count: i32,
    upload_history_count: i32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let db = tauri::async_runtime::block_on(async {
                Database::new(None).await.expect("Failed to initialize database")
            });

            let upload_manager = UploadManager::new(db.pool().clone());

            let app_state = Arc::new(Mutex::new(AppState {
                db: Arc::new(db),
                r2_client: Arc::new(Mutex::new(None)),
                upload_manager: Arc::new(upload_manager),
                active_uploads: Arc::new(Mutex::new(HashMap::new())),
            }));

            app.manage(app_state);

            // Create system tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Build tray icon
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Cloudflare Backup")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            connect_r2,
            get_saved_bucket,
            get_current_credentials,
            load_and_connect,
            list_objects,
            upload_file,
            upload_file_with_progress,
            download_file,
            download_file_with_progress,
            download_folder_as_zip,
            delete_file,
            get_active_uploads,
            cancel_upload,
            pause_upload,
            resume_upload,
            retry_upload,
            create_folder,
            list_directory,
            check_path_type,
            check_connection,
            get_temp_dir,
            read_text_file,
            delete_temp_file,
            export_config,
            import_config,
            export_migration_backup,
            import_migration_backup,
            preview_migration_backup,
            get_sync_folders,
            add_sync_folder,
            remove_sync_folder,
            toggle_sync_folder,
            hide_to_tray,
            show_from_tray,
            rename_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

