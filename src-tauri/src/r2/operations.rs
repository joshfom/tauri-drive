use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use anyhow::{Result, Context};
use crate::utils::R2Object;
use chrono::DateTime;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

/// Progress callback for download operations
pub type DownloadProgressCallback = Box<dyn Fn(i64, i64, f64, i64) + Send + Sync>;

pub async fn list_objects(
    client: &Client,
    bucket: &str,
    prefix: Option<&str>,
) -> Result<Vec<R2Object>> {
    let mut request = client.list_objects_v2().bucket(bucket);

    if let Some(p) = prefix {
        request = request.prefix(p);
    }

    let response = request.send().await?;
    
    let mut objects = Vec::new();

    if let Some(contents) = response.contents {
        for obj in contents {
            objects.push(R2Object {
                key: obj.key().unwrap_or("").to_string(),
                size: obj.size().unwrap_or(0),
                last_modified: obj
                    .last_modified()
                    .and_then(|dt| DateTime::parse_from_rfc3339(&dt.to_string()).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
                etag: obj.e_tag().unwrap_or("").to_string(),
                is_directory: false,
            });
        }
    }

    Ok(objects)
}

/// Download object with streaming (memory efficient for large files)
pub async fn get_object_streaming(
    client: &Client,
    bucket: &str,
    key: &str,
    local_path: &str,
    progress_callback: Option<DownloadProgressCallback>,
) -> Result<()> {
    let response = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("Failed to get object from R2")?;

    let content_length = response.content_length().unwrap_or(0);
    let mut byte_stream = response.body;
    
    // Create file for writing
    let mut file = tokio::fs::File::create(local_path)
        .await
        .context("Failed to create local file")?;

    let downloaded = Arc::new(AtomicI64::new(0));
    let start_time = std::time::Instant::now();

    // Stream chunks to file
    while let Some(chunk) = byte_stream.try_next().await? {
        file.write_all(&chunk).await?;
        
        let total_downloaded = downloaded.fetch_add(chunk.len() as i64, Ordering::SeqCst) + chunk.len() as i64;
        
        // Call progress callback if provided
        if let Some(ref callback) = progress_callback {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { total_downloaded as f64 / elapsed } else { 0.0 };
            let remaining = content_length - total_downloaded;
            let eta = if speed > 0.0 { (remaining as f64 / speed) as i64 } else { 0 };
            
            callback(total_downloaded, content_length, speed, eta);
        }
    }

    file.flush().await?;
    
    log::info!("Downloaded {} bytes to {}", content_length, local_path);
    Ok(())
}

/// Legacy get_object (kept for compatibility, but uses streaming internally now)
pub async fn get_object(
    client: &Client,
    bucket: &str,
    key: &str,
    local_path: &str,
) -> Result<()> {
    get_object_streaming(client, bucket, key, local_path, None).await
}

pub async fn put_object(
    client: &Client,
    bucket: &str,
    key: &str,
    file_path: &str,
) -> Result<String> {
    let body = ByteStream::from_path(std::path::Path::new(file_path)).await?;

    let response = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    Ok(response.e_tag().unwrap_or("").to_string())
}

/// Upload progress callback type
pub type UploadProgressCallback = Box<dyn Fn(i64, i64, f64, i64) + Send + Sync>;

/// Upload a file with progress tracking
pub async fn put_object_with_progress<F>(
    client: &Client,
    bucket: &str,
    key: &str,
    file_path: &str,
    progress_callback: F,
) -> Result<String>
where
    F: Fn(i64, i64, f64, i64) + Send + 'static,
{
    use tokio::io::AsyncReadExt;
    
    let path = std::path::Path::new(file_path);
    let file_size = tokio::fs::metadata(path).await?.len() as i64;
    
    // For very small files (< 1MB), just upload directly without chunked progress
    if file_size < 1024 * 1024 {
        let body = ByteStream::from_path(path).await?;
        let response = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .send()
            .await?;
        
        // Emit final progress
        progress_callback(file_size, file_size, 0.0, 0);
        return Ok(response.e_tag().unwrap_or("").to_string());
    }

    // For larger files, read in chunks and track progress
    let mut file = tokio::fs::File::open(path).await?;
    let mut data = Vec::with_capacity(file_size as usize);
    
    let chunk_size = 256 * 1024; // 256KB chunks for progress reporting
    let mut buffer = vec![0u8; chunk_size];
    let mut total_read: i64 = 0;
    let start_time = std::time::Instant::now();
    
    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        
        data.extend_from_slice(&buffer[..bytes_read]);
        total_read += bytes_read as i64;
        
        // Calculate speed and ETA
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { total_read as f64 / elapsed } else { 0.0 };
        let remaining = file_size - total_read;
        let eta = if speed > 0.0 { (remaining as f64 / speed) as i64 } else { 0 };
        
        // Report progress during read phase (50% of total progress)
        progress_callback(total_read / 2, file_size, speed, eta);
    }
    
    // Now upload the data
    let body = ByteStream::from(data);
    let response = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;
    
    // Emit final progress
    progress_callback(file_size, file_size, 0.0, 0);
    
    Ok(response.e_tag().unwrap_or("").to_string())
}

pub async fn put_object_from_bytes(
    client: &Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<String> {
    let body = ByteStream::from(data);

    let response = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;

    Ok(response.e_tag().unwrap_or("").to_string())
}

pub async fn delete_object(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    Ok(())
}

pub async fn copy_object(
    client: &Client,
    bucket: &str,
    source_key: &str,
    dest_key: &str,
) -> Result<()> {
    let copy_source = format!("{}/{}", bucket, source_key);
    
    client
        .copy_object()
        .bucket(bucket)
        .copy_source(copy_source)
        .key(dest_key)
        .send()
        .await?;

    Ok(())
}

/// Information about a stalled multipart upload
#[derive(Debug, Clone, serde::Serialize)]
pub struct StalledUpload {
    pub upload_id: String,
    pub key: String,
    pub initiated: String,
    pub age_hours: i64,
}

/// List all in-progress multipart uploads in the bucket
pub async fn list_multipart_uploads(
    client: &Client,
    bucket: &str,
) -> Result<Vec<StalledUpload>> {
    let response = client
        .list_multipart_uploads()
        .bucket(bucket)
        .send()
        .await
        .context("Failed to list multipart uploads")?;

    let mut uploads = Vec::new();
    let now = chrono::Utc::now();

    if let Some(upload_list) = response.uploads {
        for upload in upload_list {
            let upload_id = upload.upload_id().unwrap_or("").to_string();
            let key = upload.key().unwrap_or("").to_string();
            
            let initiated = upload.initiated()
                .map(|dt| dt.to_string())
                .unwrap_or_default();
            
            // Calculate age in hours
            let age_hours = upload.initiated()
                .and_then(|dt| {
                    DateTime::parse_from_rfc3339(&dt.to_string()).ok()
                })
                .map(|dt| {
                    let duration = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
                    duration.num_hours()
                })
                .unwrap_or(0);

            uploads.push(StalledUpload {
                upload_id,
                key,
                initiated,
                age_hours,
            });
        }
    }

    log::info!("Found {} in-progress multipart uploads", uploads.len());
    Ok(uploads)
}

/// Abort a specific multipart upload
pub async fn abort_multipart_upload(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
) -> Result<()> {
    client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .context(format!("Failed to abort multipart upload {} for key {}", upload_id, key))?;

    log::info!("Aborted multipart upload {} for key {}", upload_id, key);
    Ok(())
}

/// Clean up stalled multipart uploads older than the specified hours
/// Returns the number of uploads cleaned up
pub async fn cleanup_stalled_uploads(
    client: &Client,
    bucket: &str,
    max_age_hours: i64,
) -> Result<(i32, Vec<StalledUpload>)> {
    let uploads = list_multipart_uploads(client, bucket).await?;
    
    let stalled: Vec<StalledUpload> = uploads
        .into_iter()
        .filter(|u| u.age_hours >= max_age_hours)
        .collect();

    let mut cleaned_count = 0;
    let mut cleaned_uploads = Vec::new();

    for upload in &stalled {
        match abort_multipart_upload(client, bucket, &upload.key, &upload.upload_id).await {
            Ok(_) => {
                cleaned_count += 1;
                cleaned_uploads.push(upload.clone());
                log::info!(
                    "Cleaned up stalled upload: {} (age: {} hours)",
                    upload.key,
                    upload.age_hours
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to clean up upload {} for key {}: {}",
                    upload.upload_id,
                    upload.key,
                    e
                );
            }
        }
    }

    log::info!(
        "Cleanup complete: {} stalled uploads removed (threshold: {} hours)",
        cleaned_count,
        max_age_hours
    );

    Ok((cleaned_count, cleaned_uploads))
}

