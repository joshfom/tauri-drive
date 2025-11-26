use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use anyhow::{Result, Context};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicBool, Ordering};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::fs::File;
use tokio::sync::{Mutex, Semaphore};
use futures::future::join_all;

const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MIN_CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5MB minimum for S3
const MAX_CONCURRENT_UPLOADS: usize = 8; // 8 concurrent chunk uploads

/// Progress information for uploads
#[derive(Clone)]
pub struct UploadProgressInfo {
    pub uploaded_bytes: i64,
    pub total_bytes: i64,
    pub speed_bytes_per_sec: f64,
    pub eta_seconds: i64,
}

pub struct MultipartUpload {
    client: Client,
    bucket: String,
    key: String,
    upload_id: String,
    chunk_size: usize,
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl MultipartUpload {
    pub async fn new(
        client: Client,
        bucket: String,
        key: String,
        chunk_size: Option<usize>,
    ) -> Result<Self> {
        let response = client
            .create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = response
            .upload_id()
            .context("No upload ID returned")?
            .to_string();

        log::info!("Created multipart upload with ID: {} for key: {}", upload_id, key);

        Ok(Self {
            client,
            bucket,
            key,
            upload_id,
            chunk_size: chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(MIN_CHUNK_SIZE),
            cancelled: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn upload_id(&self) -> &str {
        &self.upload_id
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }
    
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
    
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    async fn upload_part_internal(
        client: &Client,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Vec<u8>,
    ) -> Result<String> {
        let body = ByteStream::from(data);

        let response = client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(body)
            .send()
            .await
            .context(format!("Failed to upload part {}", part_number))?;

        let etag = response.e_tag()
            .context(format!("No ETag returned for part {}", part_number))?
            .to_string();

        log::debug!("Uploaded part {} with ETag: {}", part_number, etag);
        Ok(etag)
    }

    pub async fn complete(&self, mut parts: Vec<(i32, String)>) -> Result<()> {
        // Sort parts by part number - CRITICAL for S3/R2
        parts.sort_by_key(|(num, _)| *num);

        log::info!("Completing multipart upload {} with {} parts", self.upload_id, parts.len());

        let completed_parts: Vec<CompletedPart> = parts
            .into_iter()
            .map(|(part_number, etag)| {
                CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(etag)
                    .build()
            })
            .collect();

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(&self.key)
            .upload_id(&self.upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .context("Failed to complete multipart upload")?;

        log::info!("Successfully completed multipart upload for key: {}", self.key);
        Ok(())
    }

    pub async fn abort(&self) -> Result<()> {
        log::warn!("Aborting multipart upload {} for key: {}", self.upload_id, self.key);
        
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(&self.key)
            .upload_id(&self.upload_id)
            .send()
            .await
            .context("Failed to abort multipart upload")?;

        Ok(())
    }

    /// Upload a file with concurrent chunk uploads (up to 8 parallel) and progress tracking
    pub async fn upload_file_concurrent<F>(
        &self,
        file_path: &str,
        progress_callback: F,
    ) -> Result<Vec<(i32, String)>>
    where
        F: FnMut(UploadProgressInfo) + Send + 'static,
    {
        let path = Path::new(file_path);
        let file_size = tokio::fs::metadata(path).await?.len() as i64;
        let chunk_size = self.chunk_size;

        // Calculate number of parts
        let num_parts = ((file_size as usize + chunk_size - 1) / chunk_size) as i32;
        
        log::info!(
            "Starting concurrent upload of {} ({} bytes) in {} parts with {} concurrent uploads",
            file_path, file_size, num_parts, MAX_CONCURRENT_UPLOADS
        );

        // Shared state for tracking progress
        let total_uploaded = Arc::new(AtomicI64::new(0));
        let start_time = Instant::now();
        let parts: Arc<Mutex<Vec<(i32, String)>>> = Arc::new(Mutex::new(Vec::with_capacity(num_parts as usize)));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS));
        let progress_callback = Arc::new(Mutex::new(progress_callback));

        // Read all chunks and create upload tasks
        let mut tasks = Vec::new();
        let mut file = File::open(path).await?;
        let mut part_number = 1;
        let mut offset: u64 = 0;

        while offset < file_size as u64 {
            // Check for cancellation
            if self.is_cancelled() {
                log::info!("Upload cancelled, aborting...");
                self.abort().await.ok();
                return Err(anyhow::anyhow!("Upload cancelled"));
            }
            
            // Wait while paused
            while self.is_paused() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if self.is_cancelled() {
                    log::info!("Upload cancelled while paused, aborting...");
                    self.abort().await.ok();
                    return Err(anyhow::anyhow!("Upload cancelled"));
                }
            }

            let bytes_to_read = std::cmp::min(chunk_size as u64, file_size as u64 - offset) as usize;
            let mut buffer = vec![0u8; bytes_to_read];
            
            file.seek(std::io::SeekFrom::Start(offset)).await?;
            file.read_exact(&mut buffer).await?;

            let client = self.client.clone();
            let bucket = self.bucket.clone();
            let key = self.key.clone();
            let upload_id = self.upload_id.clone();
            let sem = semaphore.clone();
            let parts_clone = parts.clone();
            let total_uploaded_clone = total_uploaded.clone();
            let progress_callback_clone = progress_callback.clone();
            let cancelled = self.cancelled.clone();
            let paused = self.paused.clone();
            let current_part = part_number;
            let chunk_len = buffer.len() as i64;

            let task = tokio::spawn(async move {
                // Acquire semaphore permit to limit concurrency
                let _permit = sem.acquire().await.unwrap();

                // Check for cancellation or wait while paused
                loop {
                    if cancelled.load(Ordering::SeqCst) {
                        return Err(anyhow::anyhow!("Upload cancelled"));
                    }
                    if !paused.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }

                log::debug!("Uploading part {} ({} bytes)", current_part, chunk_len);

                let etag = Self::upload_part_internal(
                    &client,
                    &bucket,
                    &key,
                    &upload_id,
                    current_part,
                    buffer,
                ).await?;

                // Update progress
                let uploaded = total_uploaded_clone.fetch_add(chunk_len, Ordering::SeqCst) + chunk_len;
                
                // Store completed part
                {
                    let mut parts_guard = parts_clone.lock().await;
                    parts_guard.push((current_part, etag));
                }

                // Calculate speed and ETA
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 { uploaded as f64 / elapsed } else { 0.0 };
                let remaining_bytes = file_size - uploaded;
                let eta = if speed > 0.0 { (remaining_bytes as f64 / speed) as i64 } else { 0 };

                // Call progress callback
                {
                    let mut callback = progress_callback_clone.lock().await;
                    callback(UploadProgressInfo {
                        uploaded_bytes: uploaded,
                        total_bytes: file_size,
                        speed_bytes_per_sec: speed,
                        eta_seconds: eta,
                    });
                }

                Ok::<(), anyhow::Error>(())
            });

            tasks.push(task);
            part_number += 1;
            offset += bytes_to_read as u64;
        }

        // Wait for all uploads to complete
        let results = join_all(tasks).await;
        
        // Check for errors
        for result in results {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    log::error!("Part upload failed: {}", e);
                    self.abort().await.ok();
                    return Err(e);
                }
                Err(e) => {
                    log::error!("Task panicked: {}", e);
                    self.abort().await.ok();
                    return Err(anyhow::anyhow!("Upload task panicked: {}", e));
                }
            }
        }

        // Get all completed parts
        let completed_parts = parts.lock().await.clone();
        
        if completed_parts.len() != num_parts as usize {
            self.abort().await.ok();
            return Err(anyhow::anyhow!(
                "Expected {} parts but only {} completed",
                num_parts,
                completed_parts.len()
            ));
        }

        log::info!("All {} parts uploaded successfully", completed_parts.len());
        Ok(completed_parts)
    }

    /// Legacy sequential upload (kept for compatibility)
    pub async fn upload_file_with_progress<F>(
        &self, 
        file_path: &str,
        mut progress_callback: F,
    ) -> Result<Vec<(i32, String)>>
    where
        F: FnMut(i64, i64),
    {
        let path = Path::new(file_path);
        let mut file = File::open(path).await?;
        let file_size = file.metadata().await?.len() as i64;

        let mut parts = Vec::new();
        let mut part_number = 1;
        let mut buffer = vec![0u8; self.chunk_size];
        let mut total_uploaded: i64 = 0;

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk = buffer[..bytes_read].to_vec();
            let etag = Self::upload_part_internal(
                &self.client,
                &self.bucket,
                &self.key,
                &self.upload_id,
                part_number,
                chunk,
            ).await?;
            parts.push((part_number, etag));

            total_uploaded += bytes_read as i64;
            progress_callback(total_uploaded, file_size);

            part_number += 1;
        }

        Ok(parts)
    }
}

pub async fn upload_large_file(
    client: &Client,
    bucket: &str,
    key: &str,
    file_path: &str,
    chunk_size: Option<usize>,
) -> Result<()> {
    let upload = MultipartUpload::new(
        client.clone(),
        bucket.to_string(),
        key.to_string(),
        chunk_size,
    )
    .await?;

    let parts = upload.upload_file_concurrent(file_path, |_progress| {}).await?;
    upload.complete(parts).await?;

    Ok(())
}
