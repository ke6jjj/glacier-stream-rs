use std::io::Write;

use crate::result::{Error as EasyError, Result as EasyResult};
use crate::size::SizeSpec;
use crate::util::client::get_client;
use crate::util::tree_hash::AWS_TREE_HASH_PART_SIZE;
use crate::util::tree_hash::SequentialTreeHash;
use crate::util::tree_hash::ReservedTreeHash;
use crate::util::vault::{GlacierVaultSpec, parse_glacier_vault_arn};
use anyhow::Ok;
use aws_sdk_glacier::client::Client as GlacierClient;
use sha2::{Digest, Sha256};
use tokio::task::JoinSet;

/// Stream data from a Glacier vault to stdout.
///
/// To achieve the fastest download speed you may want to stream the data
/// using more than concurrent download request. However, each download
/// that you spawn comes with a memory cost as each download part may have
/// to be stored in memory until the parts before it arrive. The `chunk_size`
/// parameter controls how much data each download worker downloads at a
/// time. Total memory required is approximately `chunk_size * workers * 2`.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Number of concurrent download workers. Default: 4
    #[arg(short, long, default_value_t = 4)]
    workers: usize,
    /// How much data each download worker should request when streaming
    /// data. Must be a multiple of 1 MB. Supports human readable units: b, k, m, g, and t.
    /// Default: 100m
    #[arg(short, long, default_value = "100m")]
    chunk_size: SizeSpec,
    /// Be verbose. Prints additional information about the download process
    /// to stderr.
    #[arg(short, long)]
    verbose: bool,
    /// ARN for the source vault.
    /// Example: arn:aws:glacier:us-east-2:123456789012:vaults/video-archives
    #[arg(value_parser = parse_glacier_vault_arn)]
    arn: GlacierVaultSpec,
    /// Job ID of the archive retrieval job.
    job_id: String,
}

impl Cmd {
    pub async fn run(&self) -> EasyResult<()> {
        self.chunk_size
            .to_bytes()
            .checked_rem(AWS_TREE_HASH_PART_SIZE)
            .ok_or_else(|| EasyError::msg("Chunk size must be a multiple of 1 MB"))?;
        let client = get_client(&self.arn).await;
        let job = self.prepare_download_job(client).await?;
        download(job, self.workers).await
    }

    async fn prepare_download_job(&self, client: GlacierClient) -> EasyResult<DownloadJob> {
        let job_info = client
            .describe_job()
            .vault_name(self.arn.vault_name())
            .job_id(&self.job_id)
            .send()
            .await?;
        job_info
            .completed()
            .then_some(())
            .ok_or_else(|| EasyError::msg("Job not finished."))?;
        let total_size = job_info
            .archive_size_in_bytes()
            .map(|s| s as u64)
            .ok_or_else(|| EasyError::msg("Job info missing archive size"))?;
        let tree_hash = job_info
            .sha256_tree_hash()
            .and_then(|h| hex::decode(h).ok())
            .and_then(|h| h.try_into().ok())
            .ok_or_else(|| EasyError::msg("Job info missing or invalid tree hash"))?;
        let chunking_size = self.chunk_size.to_bytes();
        if self.verbose {
            eprintln!("Job info:");
            eprintln!("  Archive size: {} bytes", total_size);
            eprintln!("  Tree hash: {}", hex::encode(tree_hash));
            eprintln!("  Chunking size: {} bytes", chunking_size);
        }
        let job = DownloadJob {
            client,
            vault_spec: self.arn.clone(),
            job_id: self.job_id.clone(),
            total_size,
            chunking_size,
            tree_hash,
            verbose: self.verbose,
        };
        Ok(job)
    }
}

// Context for a download operation, which can be shared across multiple
// download workers.
#[derive(Debug, Clone)]
struct DownloadJob {
    client: GlacierClient,
    vault_spec: GlacierVaultSpec,
    job_id: String,
    total_size: u64,
    chunking_size: u64,
    tree_hash: [u8; 32],
    verbose: bool,
}

#[derive(Clone)]
struct OutputWorkerJob {
    total_size: u64,
    tree_hash: [u8; 32],
    result_chan: ResultRxChannel,
    abort_chan: AbortTxChannel,
}

#[derive(Clone)]
struct DownloadWorkerJob {
    client: GlacierClient,
    vault_spec: GlacierVaultSpec,
    job_id: String,
    work_chan: WorkRxChannel,
    result_chan: ResultTxChannel,
    abort_chan: AbortTxChannel,
    verbose: bool,
}

#[derive(Debug, Clone)]
struct DownloadWorkerCommand {
    offset: u64,
    size: u64,
}

// Represents a part of the archive that has been downloaded, along with its
// byte range and data.
#[derive(Debug)]
struct DownloadedPart {
    offset: u64,
    data: Vec<u8>,
    tree_hash: ReservedTreeHash,
}

type WorkRxChannel = tokio_mpmc::Receiver<DownloadWorkerCommand>;
type WorkTxChannel = tokio_mpmc::Sender<DownloadWorkerCommand>;

struct WorkQueue {
    tx: WorkTxChannel,
    rx: WorkRxChannel,
}

impl From<(WorkTxChannel, WorkRxChannel)> for WorkQueue {
    fn from((tx, rx): (WorkTxChannel, WorkRxChannel)) -> Self {
        Self { tx, rx }
    }
}

impl WorkQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}

type ResultRxChannel = tokio_mpmc::Receiver<DownloadedPart>;
type ResultTxChannel = tokio_mpmc::Sender<DownloadedPart>;

struct ResultQueue {
    tx: ResultTxChannel,
    rx: ResultRxChannel,
}

impl From<(ResultTxChannel, ResultRxChannel)> for ResultQueue {
    fn from((tx, rx): (ResultTxChannel, ResultRxChannel)) -> Self {
        Self { tx, rx }
    }
}

impl ResultQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}


type AbortRxChannel = tokio_mpmc::Receiver<()>;
type AbortTxChannel = tokio_mpmc::Sender<()>;

struct AbortQueue {
    tx: AbortTxChannel,
    rx: AbortRxChannel,
}

impl From<(AbortTxChannel, AbortRxChannel)> for AbortQueue {
    fn from((tx, rx): (AbortTxChannel, AbortRxChannel)) -> Self {
        Self { tx, rx }
    }
}

impl AbortQueue {
    fn shutdown(&self) {
        self.tx.close();
    }
}

async fn download(job: DownloadJob, workers: usize) -> EasyResult<()> {
    let work_queue: WorkQueue = tokio_mpmc::channel(workers).into();
    let result_queue: ResultQueue = tokio_mpmc::channel(workers).into();
    let abort_queue: AbortQueue = tokio_mpmc::channel(workers).into();
    let mut download_worker_tasks = JoinSet::new();
    let mut output_worker_task = JoinSet::new();
    for _ in 0..workers {
        let worker_job = DownloadWorkerJob {
            client: job.client.clone(),
            vault_spec: job.vault_spec.clone(),
            job_id: job.job_id.clone(),
            work_chan: work_queue.rx.clone(),
            result_chan: result_queue.tx.clone(),
            abort_chan: abort_queue.tx.clone(),
            verbose: job.verbose,
        };
        download_worker_tasks.spawn(download_worker(worker_job));
    }
    let output_worker_job = OutputWorkerJob {
        total_size: job.total_size,
        tree_hash: job.tree_hash,
        result_chan: result_queue.rx.clone(),
        abort_chan: abort_queue.tx.clone(),
    };
    output_worker_task.spawn(output_worker(output_worker_job));
    for offset in (0..job.total_size).step_by(job.chunking_size as usize) {
        if !abort_queue.rx.is_empty() {
            break; // If an abort signal has been sent, stop sending more work
        }
        let size = std::cmp::min(job.chunking_size, job.total_size - offset);
        let cmd = DownloadWorkerCommand { offset, size };
        work_queue.tx.send(cmd).await?;
    }
    work_queue.shutdown();
    while let Some(res) = download_worker_tasks.join_next().await {
        res??; // Propagate any errors from workers
    }
    result_queue.shutdown();
    while let Some(res) = output_worker_task.join_next().await {
        res??; // Propagate any errors from output worker
    }
    abort_queue.shutdown();
    Ok(())
}

async fn download_worker(job: DownloadWorkerJob) -> EasyResult<()> {
    if let Err(e) = download_worker_loop(&job).await {
        job.abort_chan.send(()).await.ok(); // Signal other workers to stop
        return Err(e);
    }
    Ok(())
}

async fn download_worker_loop(job: &DownloadWorkerJob) -> EasyResult<()> {
    while let Some(command) = job.work_chan.recv().await? {
        let downloaded_part = download_part(job, command).await?;
        job.result_chan.send(downloaded_part).await?;
    }
    Ok(())
}

async fn download_part(
    job: &DownloadWorkerJob,
    cmd: DownloadWorkerCommand,
) -> EasyResult<DownloadedPart> {
    if job.verbose {
        eprintln!(
            "Downloading bytes {}-{}...",
            cmd.offset,
            cmd.offset + cmd.size - 1
        );
    }
    let output = job
        .client
        .get_job_output()
        .vault_name(job.vault_spec.vault_name())
        .job_id(&job.job_id)
        .range(format!(
            "bytes={}-{}",
            cmd.offset,
            cmd.offset + cmd.size - 1
        ))
        .send()
        .await?;
    let data = output.body.collect().await?.into_bytes().to_vec();
    if data.len() as u64 != cmd.size {
        return Err(EasyError::msg(format!(
            "Expected {} bytes but got {} bytes",
            cmd.size,
            data.len()
        )));
    }
    // Predict how this part will fit into the overall tree hash so
    // we can reserve a sub-tree that will merge perfectly with the precursor
    // tree.
    let expected_precursor_depth: i64 = match cmd.offset.eq(&0) {
        true => -1,
        false => (cmd.offset / AWS_TREE_HASH_PART_SIZE)
            .trailing_zeros() as i64,
    };
    let mut tree_hash = ReservedTreeHash::new(expected_precursor_depth);
    for chunk in data.chunks(AWS_TREE_HASH_PART_SIZE as usize) {
        let hash = Sha256::digest(chunk);
        tree_hash.insert(hash.into());
    }
    Ok(DownloadedPart {
        offset: cmd.offset,
        data,
        tree_hash,
    })
}

async fn output_worker(job: OutputWorkerJob) -> EasyResult<()> {
    if let Err(e) = output_worker_loop(&job).await {
        job.abort_chan.send(()).await.ok(); // Signal other workers to stop
        return Err(e);
    }
    Ok(())
}

async fn output_worker_loop(job: &OutputWorkerJob) -> EasyResult<()> {
    let mut tree_hasher = SequentialTreeHash::new();
    let mut total_written: u64 = 0;

    // Chunks may arrive out of order, so we store them in a map until we can
    // write them out and hash them in the correct order.
    let mut chunk_map = std::collections::BTreeMap::new();
    while let Some(part) = job.result_chan.recv().await? {
        chunk_map.insert(part.offset, part);
        // Write out any contiguous chunks starting from total_written
        while let Some(part) = chunk_map.remove(&total_written) {
            std::io::stdout().write_all(&part.data)?;
            part.tree_hash.merge_into(&mut tree_hasher)
                .expect("Merging part tree hash should never fail");
            total_written += part.data.len() as u64;
        }
    }
    if total_written != job.total_size {
        return Err(EasyError::msg(format!(
            "Expected to write {} bytes but only wrote {} bytes",
            job.total_size, total_written
        )));
    }
    let final_tree_hash = tree_hasher.finalize()?;
    if final_tree_hash != job.tree_hash {
        return Err(EasyError::msg("Tree hash mismatch"));
    }
    Ok(())
}
