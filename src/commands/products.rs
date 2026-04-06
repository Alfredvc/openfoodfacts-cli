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

async fn get(barcode: &str, client: &Client, output: &Output) -> Result<()> {
    let path = format!("/api/v2/product/{}.json", barcode);
    let body = client.get(&path, &[]).await?;

    let status = body.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
    if status == 0 {
        bail!("product not found: {}", barcode);
    }

    let product = body
        .get("product")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    output.print(&product);
    Ok(())
}

async fn search(_command: &ProductsCommand, _client: &Client, _output: &Output) -> Result<()> {
    bail!("not yet implemented")
}
