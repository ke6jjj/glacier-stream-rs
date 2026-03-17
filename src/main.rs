use clap::{Parser, Subcommand};
use glacier_stream::cmd::stream;
use glacier_stream::cmd::test;

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

fn main() {
    let cli = Cli::parse();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match cli.cmd {
            Cmd::Stream(stream_cmd) => { stream_cmd.run().await.unwrap(); },
            Cmd::Test(test_cmd) => { test_cmd.run().await.unwrap(); },
        }
    });
}
