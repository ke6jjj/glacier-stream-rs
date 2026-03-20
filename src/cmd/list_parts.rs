use crate::result::Result as EasyResult;
use crate::util::client::get_client;
use crate::util::vault::{GlacierVaultSpec, parse_glacier_vault_arn};
use aws_sdk_glacier::types::PartListElement;
use hex;
use std::convert::TryFrom;
use thiserror::Error;

/// List the parts of a multipart upload.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[arg(value_parser = parse_glacier_vault_arn)]
    /// ARN for the vault.
    /// Example: arn:aws:glacier:us-east-2:123456789012:vaults/video-archives
    arn: GlacierVaultSpec,
    /// Upload ID of the multipart upload to list parts for.
    upload_id: String,
}

struct Range {
    start: u64,
    end: u64,
}

#[derive(Debug, Error)]
enum RangeParseError {
    #[error("Invalid range format")]
    InvalidFormat,
    #[error("Invalid number: {0}")]
    InvalidNumber(#[from] std::num::ParseIntError),
}

impl TryFrom<&str> for Range {
    type Error = RangeParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split('-');
        let start_str = parts.next().ok_or(RangeParseError::InvalidFormat)?;
        let end_str = parts.next().ok_or(RangeParseError::InvalidFormat)?;
        let start = start_str.parse::<u64>()?;
        let end = end_str.parse::<u64>()?;
        Ok(Range { start, end })
    }
}

struct Part {
    range: Range,
    tree_hash: [u8; 32],
}

#[derive(Debug, Error)]
enum PartListParseError {
    #[error("Missing range in bytes")]
    MissingRange,
    #[error("Invalid range format: {0}")]
    InvalidRange(#[from] RangeParseError),
    #[error("Missing SHA256 tree hash")]
    MissingHash,
    #[error("Invalid SHA256 tree hash length")]
    InvalidHashLength,
    #[error("Hex decode error: {0}")]
    HexDecodeError(#[from] hex::FromHexError),
}

impl TryFrom<&PartListElement> for Part {
    type Error = PartListParseError;

    fn try_from(value: &PartListElement) -> Result<Self, Self::Error> {
        let range_str = value
            .range_in_bytes()
            .ok_or(PartListParseError::MissingRange)?;
        let range = Range::try_from(range_str)?;
        let hash_str = value
            .sha256_tree_hash()
            .ok_or(PartListParseError::MissingHash)?;
        let hash_bytes = hex::decode(hash_str)?;
        if hash_bytes.len() != 32 {
            return Err(PartListParseError::InvalidHashLength);
        }
        let mut tree_hash = [0u8; 32];
        tree_hash.copy_from_slice(&hash_bytes);
        Ok(Part { range, tree_hash })
    }
}

impl Cmd {
    pub async fn run(&self) -> EasyResult {
        let client = get_client(&self.arn).await;
        let mut parts: Vec<Part> = Vec::new();
        let mut next_marker = None;
        loop {
            let output = client
                .list_parts()
                .vault_name(self.arn.vault_name().to_owned())
                .upload_id(&self.upload_id)
                .set_marker(next_marker.clone())
                .send()
                .await?;
            let part_results: Vec<Result<Part, PartListParseError>> =
                output.parts().iter().map(Part::try_from).collect();
            let mut parsed_parts = part_results
                .into_iter()
                .collect::<Result<Vec<Part>, PartListParseError>>()?;
            parts.append(&mut parsed_parts);
            if let Some(marker) = output.marker() {
                if marker.eq("null") {
                    break;
                }
                next_marker = Some(marker.to_string());
            } else {
                break;
            }
        }
        for part in parts {
            let tree_hash_hex = hex::encode(part.tree_hash);
            println!(
                "Part {}: {} bytes, SHA256: {}",
                part.range.start,
                part.range.end - part.range.start + 1,
                tree_hash_hex
            );
        }
        Ok(())
    }
}
