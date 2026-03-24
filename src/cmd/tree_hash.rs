use crate::result::Result as EasyResult;
use crate::util::tree_hash::SequentialTreeHash;
use std::io::{self, Read};
use sha2::{Digest, Sha256};

/// Compute the tree hash of stdin, for comparison with uploaded archives.
///
/// AWS Glacier inventories uploads and archives using tree hashes, which are a
/// specific way of hashing data that allows for efficient verification of
/// large files. This command computes the tree hash of a local stream
/// (on stdin) so that it can be compared with the tree hash of an uploaded
/// archive, ensuring that the upload is correct.
/// 
/// Tree hashes are performed in parts and it is important that the part size
/// used for the tree hash computation matches the part size used for the
/// upload.
#[derive(Debug, clap::Parser)]
pub struct Cmd {
    #[arg(short, long, default_value_t = false)]
    /// Be verbose. Show exact part size being used.
    verbose: bool,
}

const AWS_TREE_HASH_PART_SIZE: u64 = 1024 * 1024;

impl Cmd {
    pub async fn run(&self) -> EasyResult<()> {
        let mut tree = SequentialTreeHash::new();
        let mut buffer = Vec::with_capacity(AWS_TREE_HASH_PART_SIZE as usize); // 1 MiB buffer
        let mut total_read: u64 = 0;
        loop {
            buffer.clear(); // Reuse buffer; otherwise read_to_end appends and memory grows without bound.
            let mut chunk_reader = io::stdin().take(AWS_TREE_HASH_PART_SIZE);
            let bytes_read = chunk_reader.read_to_end(&mut buffer)?;
            if bytes_read == 0 && total_read > 0 {
                break;
            }
            total_read += bytes_read as u64;
            let hash = Sha256::digest(&buffer);
            tree.insert(hash.into());
            if bytes_read == 0 {
                break;
            }
        }
        let tree_hash = tree.finalize()?;
        if self.verbose {
            println!("Total bytes read: {}", total_read);
        }
        println!("Tree hash: {}", hex::encode(tree_hash));
        Ok(())
    }
}
