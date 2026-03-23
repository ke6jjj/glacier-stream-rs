use anyhow::Ok;
use clap::{Parser, Subcommand};
use glacier_stream::cmd::list_parts;
use glacier_stream::cmd::upload;
use glacier_stream::cmd::download;
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
    Up(upload::Cmd),
    Down(download::Cmd),
    ListParts(list_parts::Cmd),
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Up(upload_cmd) => {
            upload_cmd.run().await?;
        }
        Cmd::Down(download_cmd) => {
            download_cmd.run().await?;
        }
        Cmd::ListParts(list_parts_cmd) => {
            list_parts_cmd.run().await?;
        }
    }
    Ok(())
}
