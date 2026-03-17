use crate::size::SizeSpec;
use crate::result::{Result, Error};
use std::io::{self, Read};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_glacier::client::Client as GlacierClient;
use aws_sdk_glacier::primitives::ByteStream;
use aws_sdk_glacier::operation::initiate_multipart_upload::InitiateMultipartUploadOutput;

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
        let client = self.get_client().await?;
        let part_size = crate::util::part_size_for_size(self.size.to_bytes());
        eprintln!("Using part size: {} bytes", part_size);
        let upload = self.initiate_upload(&client, part_size).await?;
        let upload_id = upload
            .upload_id()
            .ok_or_else(|| Error::msg("Failed to initiate multipart upload: missing upload ID"))?;
        eprintln!("Upload initiated. Upload ID: {}", upload_id);
        self.upload(part_size, &client, upload_id).await?;
        self.complete_upload(&client, upload_id).await?;
        eprintln!("Upload complete.");
        Ok(())
    }

    async fn get_client(&self) -> Result<GlacierClient> {
        let region = Region::new(self.region.clone());
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await;
        let client = GlacierClient::new(&config);
        Ok(client)
    }

    async fn initiate_upload(&self, client: &GlacierClient, part_size: u64) -> Result<InitiateMultipartUploadOutput> {
        let upload = client.initiate_multipart_upload()
            .vault_name(&self.vault)
            .part_size(part_size.to_string())
            .archive_description(&self.description)
            .send()
            .await?;
        Ok(upload)
    }

    async fn upload(&self, part_size: u64, client: &GlacierClient, upload_id: &str) -> Result {
        let mut buffer = Vec::new();
        for part_number in 1.. {
            let mut chunk_reader = io::stdin().take(part_size);
            let bytes_read = chunk_reader.read_to_end(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let body = ByteStream::from(buffer[..bytes_read].to_vec());
            let range_start = (part_number - 1) * part_size;
            let range_end = range_start + bytes_read as u64 - 1;
            let range_spec = format!("bytes {}-{}/*", range_start, range_end);
            client.upload_multipart_part()
                .vault_name(&self.vault)
                .upload_id(upload_id)
                .body(body)
                .range(range_spec)
                .send()
                .await?;
        }
        Ok(())
    }

    async fn complete_upload(&self, client: &GlacierClient, upload_id: &str) -> Result {
        client.complete_multipart_upload()
            .vault_name(&self.vault)
            .upload_id(upload_id)
            .send()
            .await?;
        Ok(())
    }
}