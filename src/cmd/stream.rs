use crate::size::SizeSpec;

#[derive(Debug, clap::Args)]
pub struct Stream {
    region: String,
    vault: String,
    description: String,
    size: SizeSpec,
}
