use anyhow::{bail, Result};
use crate::cli::FacetsCommand;
use crate::client::Client;
use crate::output::Output;

pub async fn run(command: &FacetsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        FacetsCommand::List { facet_type } => list(facet_type, client, output).await,
    }
}

async fn list(_facet_type: &str, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}
