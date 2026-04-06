use anyhow::Result;
use clap::Parser;

mod cli;
mod client;
mod commands;
mod output;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let msg = serde_json::json!({"error": e.to_string()});
        eprintln!("{}", serde_json::to_string(&msg).unwrap());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = output::Output::new(cli.json, cli.fields.clone());
    let client = client::Client::new()?;

    match &cli.command {
        Commands::Products { command } => commands::products::run(command, &client, &output).await,
        Commands::Facets { command } => commands::facets::run(command, &client, &output).await,
    }
}
