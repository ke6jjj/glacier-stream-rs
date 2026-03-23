use crate::util::vault::{parse_glacier_vault_arn, GlacierVaultSpec};
use crate::size::SizeSpec;

/// Stream data from a Glacier vault to stdout.
///
/// To achieve the fastest download speed you may want to stream the data
/// using more than concurrent download request. However, each download
/// that you spawn comes with a memory cost as each download part may have
/// to be stored in memory until the parts before it arrive 
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Number of concurrent download workers. Default: 4
    #[arg(short, long, default_value_t = 4)]
    workers: usize,
    /// Size of each download part. Human readable sizes are supported.
    /// Units: b, k, m, g, and t. Default: 100m
    #[arg(short, long, default_value = "100m")]
    part_size: SizeSpec,
    /// Be verbose. Prints additional information about the upload process.
    #[arg(short, long)]
    verbose: bool,
    /// ARN for the source vault.
    /// Example: arn:aws:glacier:us-east-2:123456789012:vaults/video-archives
    #[arg(value_parser = parse_glacier_vault_arn)]
    arn: GlacierVaultSpec,
    /// Archive ID of the archive to download.
    archive_id: String,
}

impl Cmd {
    pub async fn run(&self) -> crate::result::Result<()> {
        eprintln!("Not yet");
        Ok(())
    }
}
