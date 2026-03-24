use crate::result::{Error as EasyError, Result as EasyResult};
use crate::size::SizeSpec;
use crate::util::client::get_client;
use crate::util::tree_hash::RandomInsertTreeHash;
use crate::util::vault::{GlacierVaultSpec, parse_glacier_vault_arn};
use aws_sdk_glacier::client::Client as GlacierClient;
use aws_sdk_glacier::operation::complete_multipart_upload::CompleteMultipartUploadOutput;
use aws_sdk_glacier::primitives::ByteStream;
use std::io::{self, Read};
use thiserror::Error;
use tokio::task::JoinSet;
use tokio_mpmc::channel;

/// Stream data to a Glacier vault from stdin.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Number of concurrent upload workers. Default: 4
    #[arg(short, long, default_value_t = 4)]
    workers: usize,
    /// Be verbose. Prints additional information about the upload process.
    #[arg(short, long)]
    verbose: bool,
    #[arg(value_parser = parse_glacier_vault_arn)]
    /// ARN for the destination vault.
    /// Example: arn:aws:glacier:us-east-2:123456789012:vaults/video-archives
    arn: GlacierVaultSpec,
    /// A description of the archive being uploaded.
    description: String,
    /// Estimated size of upload. Human readable sizes are supported.
    /// Units: b, k, m, g, and t. Example: 1.1t
    size: SizeSpec,
}

// Represents a single part to be uploaded, including the
// byte range and the data to be uploaded.
#[derive(Debug)]
struct UploadPart {
    range_start: u64,
    range_end: u64,
    body: ByteStream,
}

// Represents the result of uploading a part, especially the
// checksum, which is needed to compute the overall tree hash for the upload.
#[derive(Debug)]
struct UploadResult {
    range_start: u64,
    range_end: u64,
    checksum: [u8; 32],
}

#[derive(Debug, Clone)]
struct UploadContext {
    client: GlacierClient,
    vault: String,
    upload_id: String,
    part_size: u64,
}

type WorkRxChannel = tokio_mpmc::Receiver<UploadPart>;
type WorkTxChannel = tokio_mpmc::Sender<UploadPart>;

struct WorkQueue {
    tx: WorkTxChannel,
    rx: WorkRxChannel,
}

impl From<(WorkTxChannel, WorkRxChannel)> for WorkQueue {
    fn from(channels: (WorkTxChannel, WorkRxChannel)) -> Self {
        WorkQueue {
            tx: channels.0,
            rx: channels.1,
        }
    }
}

impl WorkQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}

type ResultTxChannel = tokio_mpmc::Sender<UploadResult>;
type ResultRxChannel = tokio_mpmc::Receiver<UploadResult>;

struct ResultQueue {
    tx: ResultTxChannel,
    rx: ResultRxChannel,
}

impl From<(ResultTxChannel, ResultRxChannel)> for ResultQueue {
    fn from(channels: (ResultTxChannel, ResultRxChannel)) -> Self {
        ResultQueue {
            tx: channels.0,
            rx: channels.1,
        }
    }
}

impl ResultQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}

type AbortTxChannel = tokio_mpmc::Sender<()>;
type AbortRxChannel = tokio_mpmc::Receiver<()>;

struct AbortQueue {
    tx: AbortTxChannel,
    rx: AbortRxChannel,
}

impl From<(AbortTxChannel, AbortRxChannel)> for AbortQueue {
    fn from(channels: (AbortTxChannel, AbortRxChannel)) -> Self {
        AbortQueue {
            tx: channels.0,
            rx: channels.1,
        }
    }
}

impl AbortQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}


#[derive(Clone)]
struct UploadWorkerContext {
    upload: UploadContext,
    work_queue: WorkRxChannel,
    result_queue: ResultTxChannel,
    abort_semaphore: AbortTxChannel,
}

#[derive(Debug, Error)]
pub enum ReadWorkerError {
    #[error("I/O error while reading input: {0}")]
    Io(#[from] io::Error),
    #[error("Upload aborted due to an error in a worker task")]
    UploadAborted,
    #[error("Failed to send work to upload worker:")]
    SendWorkFailed,
}
struct Uploader {
    upload_context: UploadContext,
    workers: usize,
    work_queue: WorkQueue,
    result_queue: ResultQueue,
    abort_queue: AbortQueue,
    worker_tasks: JoinSet<EasyResult>,
    hash_task: JoinSet<EasyResult<[u8; 32]>>,
}

impl<'a> Uploader {
    pub fn new(workers: usize, upload_context: &UploadContext) -> Self {
        let work_queue: WorkQueue = channel::<UploadPart>(workers).into();
        let result_queue: ResultQueue = channel::<UploadResult>(workers).into();
        let abort_queue: AbortQueue = channel::<()>(1).into();
        Uploader {
            upload_context: upload_context.clone(),
            workers,
            work_queue,
            result_queue,
            abort_queue,
            worker_tasks: JoinSet::new(),
            hash_task: JoinSet::new(),
        }
    }

    // Return a queue to which the read worker can send upload parts for
    // processing by the upload workers.
    pub fn work_tx_queue(&'a self) -> &'a WorkTxChannel {
        &self.work_queue.tx
    }

    // Return a semaphone/queue that workers can post to in order to signal
    // that the upload should be aborted.
    pub fn abort_rx_queue(&'a self) -> &'a AbortRxChannel {
        &self.abort_queue.rx
    }

    pub async fn start(&mut self) {
        for _ in 0..self.workers {
            let work_context = UploadWorkerContext {
                upload: self.upload_context.clone(),
                work_queue: self.work_queue.rx.clone(),
                result_queue: self.result_queue.tx.clone(),
                abort_semaphore: self.abort_queue.tx.clone(),
            };
            self.worker_tasks.spawn(upload_worker(work_context.clone()));
        }
        self.hash_task.spawn(tree_hash_worker(
            self.result_queue.rx.clone(),
            self.abort_queue.tx.clone(),
            self.upload_context.part_size,
        ));
    }

    pub async fn finish(mut self) -> EasyResult<[u8; 32]> {
        self.work_queue.shutdown();
        let results = self.worker_tasks.join_all().await;
        for result in results {
            result?;
        }
        self.result_queue.shutdown();
        let hash_result = self.hash_task.join_next().await.unwrap()?;
        self.abort_queue.shutdown();
        hash_result
    }
}

async fn upload_worker(work_context: UploadWorkerContext) -> EasyResult<()> {
    let res = upload_loop(work_context.clone()).await;
    if let Err(e) = res {
        work_context.abort_semaphore.send(()).await?;
        return Err(e);
    }
    Ok(())
}

async fn upload_loop(work_context: UploadWorkerContext) -> EasyResult<()> {
    while let Some(part) = work_context.work_queue.recv().await? {
        let range_spec = format!("bytes {}-{}/*", part.range_start, part.range_end);
        let result = work_context
            .upload
            .client
            .upload_multipart_part()
            .vault_name(&work_context.upload.vault)
            .upload_id(&work_context.upload.upload_id)
            .body(part.body)
            .range(range_spec)
            .send()
            .await?;
        let checksum_hex = result
            .checksum()
            .ok_or_else(|| EasyError::msg("Upload part response missing checksum"))?;
        let checksum_bytes = hex::decode(checksum_hex)?;
        let report = UploadResult {
            range_start: part.range_start,
            range_end: part.range_end,
            checksum: checksum_bytes
                .try_into()
                .map_err(|_| EasyError::msg("Invalid checksum length"))?,
        };
        work_context.result_queue.send(report).await?;
    }
    Ok(())
}

async fn tree_hash_worker(
    chan: ResultRxChannel,
    abort_chan: AbortTxChannel,
    part_size: u64,
) -> EasyResult<[u8; 32]> {
    let mut tree_hash = RandomInsertTreeHash::new(part_size);
    let res = tree_hash_loop(chan, &mut tree_hash).await;
    if let Err(e) = res {
        abort_chan.send(()).await?;
        return Err(e);
    }
    tree_hash.compute_hash().map_err(|e| e.into())
}

async fn tree_hash_loop(chan: ResultRxChannel, tree_hash: &mut RandomInsertTreeHash) -> EasyResult {
    while let Some(part) = chan.recv().await? {
        tree_hash.try_insert(part.range_start, part.range_end + 1, part.checksum)?;
    }
    Ok(())
}

impl Cmd {
    pub async fn run(&self) -> EasyResult<()> {
        let size_estimate = self.size.to_bytes();
        let part_size = crate::util::part_size_for_size(size_estimate);
        if self.verbose {
            eprintln!("Estimated upload size: {} bytes", size_estimate);
        }
        if self.verbose {
            eprintln!("Using part size: {} bytes", part_size);
        }
        let client = get_client(&self.arn).await;
        let upload_id = self.initiate_upload(&client, part_size).await?;
        let context = UploadContext {
            client,
            vault: self.arn.vault_name().to_owned(),
            upload_id,
            part_size,
        };
        eprintln!("Upload ID: {}", context.upload_id);
        if self.verbose {
            eprint!("Using {} workers.", self.workers);
        }
        let result = self.upload(&context).await?;
        if self.verbose {
            eprint!(
                "Archive tree checksum: {}",
                result.checksum().unwrap_or("??NOT-PRESENT??")
            );
        }
        eprint!(
            "Archive ID: {}",
            result.archive_id().unwrap_or("??NOT-PRESENT??")
        );
        Ok(())
    }

    async fn initiate_upload(&self, client: &GlacierClient, part_size: u64) -> EasyResult<String> {
        let upload = client
            .initiate_multipart_upload()
            .vault_name(self.arn.vault_name().to_owned())
            .part_size(part_size.to_string())
            .archive_description(&self.description)
            .send()
            .await?;
        let upload_id = upload.upload_id().ok_or_else(|| {
            EasyError::msg("Failed to initiate multipart upload: missing upload ID")
        })?;
        Ok(upload_id.to_string())
    }

    async fn upload(
        &self,
        upload_context: &UploadContext,
    ) -> EasyResult<CompleteMultipartUploadOutput> {
        let mut uploader = Uploader::new(self.workers, upload_context);

        uploader.start().await;
        let tx = uploader.work_tx_queue();
        let abort = uploader.abort_rx_queue();
        let total_size = self.read_worker(upload_context, tx, abort).await?;
        let checksum = uploader.finish().await?;
        let output = self
            .complete_upload(upload_context, checksum, total_size)
            .await?;
        Ok(output)
    }

    async fn read_worker(
        &self,
        upload_context: &UploadContext,
        tx: &WorkTxChannel,
        abort: &AbortRxChannel,
    ) -> Result<u64, ReadWorkerError> {
        let mut buffer = Vec::new();
        let mut total_read: u64 = 0;
        loop {
            if !abort.is_empty() {
                return Err(ReadWorkerError::UploadAborted);
            }

            buffer.clear(); // Reuse buffer; otherwise read_to_end appends and memory grows without bound.
            let mut chunk_reader = io::stdin().take(upload_context.part_size);
            let bytes_read = chunk_reader.read_to_end(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let body = ByteStream::from(buffer[..bytes_read].to_vec());
            let range_start = total_read;
            let range_end = total_read + bytes_read as u64 - 1;
            let part = UploadPart {
                range_start,
                range_end,
                body,
            };
            total_read += bytes_read as u64;
            tx.send(part)
                .await
                .map_err(|_| ReadWorkerError::SendWorkFailed)?;
        }
        Ok(total_read)
    }

    async fn complete_upload(
        &self,
        upload_context: &UploadContext,
        checksum: [u8; 32],
        total_size: u64,
    ) -> EasyResult<CompleteMultipartUploadOutput> {
        let x = upload_context
            .client
            .complete_multipart_upload()
            .vault_name(&upload_context.vault)
            .upload_id(&upload_context.upload_id)
            .archive_size(total_size.to_string())
            .checksum(hex::encode(checksum))
            .send()
            .await?;
        Ok(x)
    }
}
