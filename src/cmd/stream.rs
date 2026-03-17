use crate::size::SizeSpec;
use clap::Parser;

/// Stream data to a Glacier vault.
#[derive(Debug, Parser)]
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
