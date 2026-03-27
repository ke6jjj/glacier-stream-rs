use crate::result::Result as EasyResult;
use crate::util::tree_hash::AWS_TREE_HASH_PART_SIZE;
use crate::util::tree_hash::SequentialTreeHash;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

/// Compute the tree hash of stdin, for comparison with uploaded archives.
///
/// AWS Glacier inventories uploads and archives using tree hashes, which are a
/// specific way of hashing data that allows for efficient verification of
/// large files. This command computes the tree hash of a local stream
/// (on stdin) so that it can be compared with the tree hash of an uploaded
/// archive, ensuring that the upload is correct.
///
/// Since tree hashes are computed in parts, the full computation can be
/// parallelized. This command allows you to specify the number of concurrent
/// hash workers to speed up the computation, especially in situationes where
/// the input data is large and the system has multiple CPU cores available.
#[derive(Debug, clap::Parser)]
pub struct Cmd {
    /// Number of concurrent hash workers. Default: 4
    #[arg(short, long, default_value_t = 4)]
    workers: usize,
    #[arg(short, long, default_value_t = false)]
    /// Be verbose. Show total bytes read as well.
    verbose: bool,
}

impl Cmd {
    pub async fn run(&self) -> EasyResult<()> {
        let work_queue = WorkQueue::from(tokio_mpmc::channel(self.workers));
        let result_queue = ResultQueue::from(tokio_mpmc::channel(self.workers));
        let collector_task = tokio::spawn(hash_collector(result_queue.rx.clone()));
        // Spawn hash workers
        let mut worker_tasks = tokio::task::JoinSet::new();
        for _ in 0..self.workers {
            worker_tasks.spawn(
                hash_worker(work_queue.rx.clone(), result_queue.tx.clone())
            );
        }
        let mut total_read: u64 = 0;
        loop {
            let mut buffer = Vec::with_capacity(AWS_TREE_HASH_PART_SIZE as usize); // 1 MiB buffer
            let mut chunk_reader = io::stdin().take(AWS_TREE_HASH_PART_SIZE);
            let bytes_read = chunk_reader.read_to_end(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            work_queue.tx.send(HashWorkerCommand {
                offset: total_read,
                data: buffer,
            }).await?;
            total_read += bytes_read as u64;
        }
        work_queue.tx.close();
        if self.verbose {
            println!("Total bytes read: {}", total_read);
        }
        while let Some(worker) = worker_tasks.join_next().await {
            worker??;
        }
        result_queue.tx.close();
        let tree_hash = collector_task.await??;
        println!("Tree hash: {}", hex::encode(tree_hash));
        Ok(())
    }
}

struct HashWorkerCommand {
    offset: u64,
    data: Vec<u8>,
}

type WorkRxChannel = tokio_mpmc::Receiver<HashWorkerCommand>;
type WorkTxChannel = tokio_mpmc::Sender<HashWorkerCommand>;

struct WorkQueue {
    tx: WorkTxChannel,
    rx: WorkRxChannel,
}

impl From<(WorkTxChannel, WorkRxChannel)> for WorkQueue {
    fn from((tx, rx): (WorkTxChannel, WorkRxChannel)) -> Self {
        Self { tx, rx }
    }
}

struct HashResult {
    offset: u64,
    hash: [u8; 32],
}

type ResultRxChannel = tokio_mpmc::Receiver<HashResult>;
type ResultTxChannel = tokio_mpmc::Sender<HashResult>;

struct ResultQueue {
    tx: ResultTxChannel,
    rx: ResultRxChannel,
}

impl From<(ResultTxChannel, ResultRxChannel)> for ResultQueue {
    fn from((tx, rx): (ResultTxChannel, ResultRxChannel)) -> Self {
        Self { tx, rx }
    }
}

async fn hash_worker(work_chan: WorkRxChannel, result_chan: ResultTxChannel) -> EasyResult<()> {
    while let Some(command) = work_chan.recv().await? {
        let mut hasher = Sha256::new();
        hasher.update(&command.data);
        let hash_result = HashResult {
            offset: command.offset,
            hash: hasher.finalize().into(),
        };
        result_chan.send(hash_result).await?;
    }
    Ok(())
}

async fn hash_collector(result_chan: ResultRxChannel) -> EasyResult<[u8; 32]> {
    let mut tree_hasher = SequentialTreeHash::new();
    let mut total_hashed: u64 = 0;
    // Chunks may arrive out of order, so we store them in a map until we can
    // write them out and hash them in the correct order.
    let mut chunk_map = std::collections::BTreeMap::new();
    while let Some(part) = result_chan.recv().await? {
        chunk_map.insert(part.offset, part);
        // Write out any contiguous chunks starting from total_hashed
        while let Some(part) = chunk_map.remove(&total_hashed) {
            tree_hasher.insert(part.hash);
            total_hashed += AWS_TREE_HASH_PART_SIZE;
        }
    }
    Ok(tree_hasher.finalize()?)
}
