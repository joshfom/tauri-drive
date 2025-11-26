use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, Config};
use aws_credential_types::Credentials;
use anyhow::Result;

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

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .credentials_provider(credentials)
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
