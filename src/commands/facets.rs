use anyhow::{bail, Result};
use crate::cli::FacetsCommand;
use crate::client::Client;
use crate::output::Output;

const VALID_FACET_TYPES: &[&str] = &[
    "categories",
    "labels",
    "ingredients",
    "brands",
    "countries",
    "additives",
    "allergens",
    "packaging",
];

pub async fn run(command: &FacetsCommand, client: &Client, output: &Output) -> Result<()> {
    match command {
        FacetsCommand::List { facet_type } => list(facet_type, client, output).await,
    }
}

async fn list(facet_type: &str, client: &Client, output: &Output) -> Result<()> {
    if !VALID_FACET_TYPES.contains(&facet_type) {
        bail!(
            "unknown facet type: \"{}\" — valid: {}",
            facet_type,
            VALID_FACET_TYPES.join(", ")
        );
    }

    let path = format!("/{}.json", facet_type);
    let body = client.get(&path, &[]).await?;

    let tags = body
        .get("tags")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    output.print(&tags);
    Ok(())
}
