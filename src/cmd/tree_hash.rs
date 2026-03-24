use crate::size::SizeSpec;
use crate::util::part_size_for_size;
use crate::result::{Result as EasyResult, Error as EasyError};

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
    #[arg(short, long, name = "exact")]
    /// Exact part size to use for the tree hash computation.
    part_size: Option<u64>,
    #[arg(short, long, name = "estimate")]
    /// Compute part size from the total estimated size of the upload.
    /// 
    /// This option will compute the part size using the same formula that
    /// the "up" command uses to compute the part size when uploading a
    /// stream. Use this option when the part size isn't exactly known but
    /// it is known that this tool was used for the upload.
    size_estimate: Option<SizeSpec>,
}

impl Cmd {
    pub async fn run(&self) -> EasyResult<()> {
        if self.part_size.is_some() && self.size_estimate.is_some() {
            eprintln!("Cannot specify both part size and size estimate");
            std::process::exit(1);
        }
        let part_size = self.part_size
            .or_else(|| {
                self.size_estimate.as_ref().map(
                    |size_estimate| part_size_for_size(size_estimate.to_bytes())
                )
            })
            .ok_or(
                EasyError::msg("Must specify either part size or size estimate")
            )?;
        eprintln!("Part size: {}", part_size);
        Ok(())
    }
}
