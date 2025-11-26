use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use anyhow::{Result, Context};
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::fs::File;

const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MIN_CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5MB minimum for S3

pub struct MultipartUpload {
    client: Client,
    bucket: String,
    key: String,
    upload_id: String,
    chunk_size: usize,
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

        Ok(Self {
            client,
            bucket,
            key,
            upload_id,
            chunk_size: chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(MIN_CHUNK_SIZE),
        })
    }

    pub fn upload_id(&self) -> &str {
        &self.upload_id
    }

    pub async fn upload_part(
        &self,
        part_number: i32,
        data: Vec<u8>,
    ) -> Result<String> {
        let body = ByteStream::from(data);

        let response = self
            .client
            .upload_part()
            .bucket(&self.bucket)
            .key(&self.key)
            .upload_id(&self.upload_id)
            .part_number(part_number)
            .body(body)
            .send()
            .await
            .context(format!("Failed to upload part {}", part_number))?;

        Ok(response.e_tag().unwrap_or("").to_string())
    }

    pub async fn complete(
        &self,
        parts: Vec<(i32, String)>,
    ) -> Result<()> {
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

        Ok(())
    }

    pub async fn abort(&self) -> Result<()> {
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

    pub async fn upload_file(&self, file_path: &str) -> Result<Vec<(i32, String)>> {
        let path = Path::new(file_path);
        let mut file = File::open(path).await?;
        let _file_size = file.metadata().await?.len() as usize;

        let mut parts = Vec::new();
        let mut part_number = 1;
        let mut buffer = vec![0u8; self.chunk_size];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let chunk = buffer[..bytes_read].to_vec();
            let etag = self.upload_part(part_number, chunk).await?;
            parts.push((part_number, etag));

            part_number += 1;
        }

        Ok(parts)
    }

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
            let etag = self.upload_part(part_number, chunk).await?;
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

    let parts = upload.upload_file(file_path).await?;
    upload.complete(parts).await?;

    Ok(())
}
