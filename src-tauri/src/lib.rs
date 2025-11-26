mod r2;
mod db;
mod utils;
mod upload;

use r2::R2Client;
use db::Database;
use upload::UploadManager;
use utils::{R2Object, R2Credentials, UploadProgress, UploadStatus};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};

pub struct AppState {
    pub db: Arc<Database>,
    pub r2_client: Arc<Mutex<Option<R2Client>>>,
    pub upload_manager: Arc<UploadManager>,
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

    r2::operations::get_object(
        client.client(),
        client.bucket(),
        &remote_key,
        &local_path,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
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

    // Emit initial progress event
    let file_name = std::path::Path::new(&local_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
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
    app_state.upload_manager
        .update_upload_status(&upload_id, "uploading", None, None)
        .await
        .map_err(|e| e.to_string())?;

    // Clone variables for closure
    let upload_id_clone = upload_id.clone();
    let file_name_clone = file_name.clone();
    let local_path_clone = local_path.clone();
    let remote_key_clone = remote_key.clone();

    // Use multipart upload for files > 100MB
    if file_size > 100 * 1024 * 1024 {
        let app_clone = app.clone();
        let upload = r2::multipart::MultipartUpload::new(
            client.client().clone(),
            client.bucket().to_string(),
            remote_key.clone(),
            Some(10 * 1024 * 1024),
        )
        .await
        .map_err(|e| e.to_string())?;

        let parts = upload
            .upload_file_with_progress(&local_path, |uploaded, total| {
                let progress_pct = (uploaded as f64 / total as f64) * 100.0;
                let progress_event = UploadProgress {
                    id: upload_id_clone.clone(),
                    file_name: file_name_clone.clone(),
                    file_path: local_path_clone.clone(),
                    remote_path: remote_key_clone.clone(),
                    total_size: total,
                    uploaded_size: uploaded,
                    progress: progress_pct,
                    speed: 0.0,
                    eta: 0,
                    status: UploadStatus::Uploading,
                    error_message: None,
                };
                app_clone.emit("upload-progress", &progress_event).ok();
            })
            .await
            .map_err(|e| {
                // Mark as failed
                let upload_manager = app_state.upload_manager.clone();
                let id = upload_id.clone();
                let err_msg = e.to_string();
                tauri::async_runtime::spawn(async move {
                    upload_manager.update_upload_status(&id, "failed", None, Some(&err_msg)).await.ok();
                });
                e.to_string()
            })?;

        upload
            .complete(parts)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        r2::operations::put_object(
            client.client(),
            client.bucket(),
            &remote_key,
            &local_path,
        )
        .await
        .map_err(|e| {
            let upload_manager = app_state.upload_manager.clone();
            let id = upload_id.clone();
            let err_msg = e.to_string();
            tauri::async_runtime::spawn(async move {
                upload_manager.update_upload_status(&id, "failed", None, Some(&err_msg)).await.ok();
            });
            e.to_string()
        })?;
    }

    // Mark as completed
    app_state.upload_manager
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
    Ok(())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db = tauri::async_runtime::block_on(async {
                Database::new(None).await.expect("Failed to initialize database")
            });

            let upload_manager = UploadManager::new(db.pool().clone());

            let app_state = Arc::new(Mutex::new(AppState {
                db: Arc::new(db),
                r2_client: Arc::new(Mutex::new(None)),
                upload_manager: Arc::new(upload_manager),
            }));

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            connect_r2,
            get_saved_bucket,
            load_and_connect,
            list_objects,
            upload_file,
            upload_file_with_progress,
            download_file,
            delete_file,
            get_active_uploads,
            cancel_upload,
            create_folder,
            list_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

