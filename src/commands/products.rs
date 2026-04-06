use anyhow::{bail, Result};
use crate::cli::ProductsCommand;
use crate::client::Client;
use crate::output::Output;

pub async fn run(command: &ProductsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        ProductsCommand::Get { barcode } => get(barcode, client, output).await,
        ProductsCommand::Search { .. } => search(command, client, output).await,
    }
}

async fn get(_barcode: &str, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}

async fn search(_command: &ProductsCommand, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}
