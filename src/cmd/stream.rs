use crate::size::SizeSpec;
use crate::result::{Result, Error};
use std::io::{self, Read};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_glacier::client::Client as GlacierClient;
use aws_sdk_glacier::primitives::ByteStream;
use aws_sdk_glacier::operation::initiate_multipart_upload::InitiateMultipartUploadOutput;
use tokio_mpmc::channel;
use tokio::task::JoinSet;

/// Stream data to a Glacier vault.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Number of concurrent upload workers. Default: 4
    #[arg(short, long, default_value_t = 4)]
    workers: usize,

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

#[derive(Debug)]
struct UploadPart {
    range_start: u64,
    range_end: u64,
    body: ByteStream,
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
        eprint!("Using {} workers.", self.workers);
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
        let (tx, rx) = channel::<UploadPart>(self.workers);
        let mut worker_tasks = JoinSet::new();
        for _ in 0..self.workers {
            let client_clone = client.clone();
            let vault_clone = self.vault.clone();
            let upload_id_clone = upload_id.to_string();
            worker_tasks.spawn(upload_worker(rx.clone(), client_clone, vault_clone, upload_id_clone));
        }
        for part_number in 1.. {
            let mut chunk_reader = io::stdin().take(part_size);
            let bytes_read = chunk_reader.read_to_end(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let body = ByteStream::from(buffer[..bytes_read].to_vec());
            let range_start = (part_number - 1) * part_size;
            let range_end = range_start + bytes_read as u64 - 1;
            let part = UploadPart {
                range_start,
                range_end,
                body,
            };
            tx.send(part).await?;
        }
        drop(tx);
        while worker_tasks.join_next().await.is_some() {}
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

async fn upload_worker<'a>(chan:tokio_mpmc::Receiver<UploadPart>, client: GlacierClient, vault: String, upload_id: String) -> Result {
    while let Some(part) = chan.recv().await? {
        let range_spec = format!("bytes {}-{}/*", part.range_start, part.range_end);
        client.upload_multipart_part()
            .vault_name(&vault)
            .upload_id(&upload_id)
            .body(part.body)
            .range(range_spec)
            .send()
            .await?;
    }
    Ok(())
}