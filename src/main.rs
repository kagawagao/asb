mod aapt2;
mod aar;
mod builder;
mod cache;
mod cli;
mod dependency;
mod types;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    cli.run().await
}
