use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use anyhow::Result;
use crate::utils::R2Object;
use chrono::DateTime;

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

pub async fn get_object(
    client: &Client,
    bucket: &str,
    key: &str,
    local_path: &str,
) -> Result<()> {
    let response = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let data = response.body.collect().await?;
    tokio::fs::write(local_path, data.into_bytes()).await?;

    Ok(())
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
