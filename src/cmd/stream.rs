use crate::size::SizeSpec;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cmd {
    region: String,
    vault: String,
    description: String,
    size: SizeSpec,
}
