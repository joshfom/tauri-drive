use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, Config};
use aws_credential_types::Credentials;
use aws_smithy_runtime::client::http::hyper_014::HyperClientBuilder;
use aws_smithy_types::timeout::TimeoutConfig;
use anyhow::Result;
use std::time::Duration;

pub struct R2Client {
    client: Client,
    bucket: String,
}

impl R2Client {
    pub async fn new(
        account_id: &str,
        access_key_id: &str,
        secret_access_key: &str,
        bucket: &str,
    ) -> Result<Self> {
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "r2-credentials",
        );

        // Configure timeouts for large file uploads
        let timeout_config = TimeoutConfig::builder()
            .operation_timeout(Duration::from_secs(300))      // 5 minutes per operation
            .operation_attempt_timeout(Duration::from_secs(120)) // 2 minutes per attempt
            .connect_timeout(Duration::from_secs(30))         // 30 seconds to connect
            .read_timeout(Duration::from_secs(60))            // 60 seconds read timeout
            .build();

        // Build HTTP client with keep-alive and proper timeouts
        let http_client = HyperClientBuilder::new().build_https();

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
            .timeout_config(timeout_config)
            .http_client(http_client)
            .build();

        let client = Client::from_conf(config);

        Ok(Self {
            client,
            bucket: bucket.to_string(),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}
