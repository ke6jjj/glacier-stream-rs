use crate::size::SizeSpec;
use crate::result::{Result, Error};
use std::io::Read;
use aws_config::BehaviorVersion;
use aws_sdk_glacier::client::Client as GlacierClient;
use aws_sdk_glacier::primitives::ByteStream;

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
        let part_size = crate::util::part_size_for_size(self.size.to_bytes());
        eprintln!("Using part size: {} bytes", part_size);
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = GlacierClient::new(&config);
        let upload = client.initiate_multipart_upload()
            .vault_name(&self.vault)
            .archive_description(&self.description)
            .part_size(format!("{}", part_size as i64))
            .send()
            .await?;
        let upload_id = upload
            .upload_id()
            .ok_or_else(|| Error::msg("Failed to initiate multipart upload: missing upload ID"))?;
        eprintln!("Upload initiated. Upload ID: {}", upload_id);
        let mut buffer = vec![0u8; part_size as usize];
        let mut stdin_handle = std::io::stdin().lock();
        for part_number in 1.. {
            let bytes_read = stdin_handle.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let body = ByteStream::from(buffer[..bytes_read].to_vec());
            let range_start = (part_number - 1) * part_size;
            let range_end = range_start + bytes_read as u64 - 1;
            client.upload_multipart_part()
                .vault_name(&self.vault)
                .upload_id(upload_id)
                .body(body)
                .range(format!("bytes {}-{}/*", range_start, range_end))
                .send()
                .await?;
        }
        client.complete_multipart_upload()
            .vault_name(&self.vault)
            .upload_id(upload_id)
            .send()
            .await?;
        eprintln!("Upload complete.");
        Ok(())
    }
}