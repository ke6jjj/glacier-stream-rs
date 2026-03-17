use anyhow::Ok;
use clap::{Parser, Subcommand};
use glacier_stream::cmd::stream;
use glacier_stream::cmd::test;
use glacier_stream::result::Result;

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    Stream(stream::Cmd),
    Test(test::Cmd),
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Stream(stream_cmd) => { stream_cmd.run().await?; },
        Cmd::Test(test_cmd) => { test_cmd.run().await?; },
    }
    Ok(())
}
