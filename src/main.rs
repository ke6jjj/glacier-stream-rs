use clap::{Parser, Subcommand};
use glacier_stream::cmd::stream;

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
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Stream(stream_cmd) => {
            println!("Stream command: {:?}", stream_cmd);
        }
    }
}
