use crate::size::SizeSpec;
use crate::result::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_glacier::client::Client as GlacierClient;

/// Stream data to a Glacier vault.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// AWS region in which the destination vault resides. Example: us-west-1
    region: String,
    /// Destination vault identifier. NOT FULL ARN, only last part. 
    /// Example: photos-audio
    vault: String,
    /// A description of the archive being uploaded.
    description: String,
    /// Estimated size of upload. Human readable sizes are supported.
    /// Units: b, k, m, g, and t. Example: 1.1t
    size: SizeSpec,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        let region = Region::new(self.region.clone());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let _client = GlacierClient::new(&config);
        eprint!("Region: {:?}, Vault: {}, Description: {}, Size: {} bytes", config.region(), self.vault, self.description, self.size.to_bytes());
        Ok(())
    }
}