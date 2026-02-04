mod aapt2;
mod aar;
mod builder;
mod cache;
mod cli;
mod dependency;
mod resource_priority;
mod types;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging - output to both console and file
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create log file
    // let log_file = File::create("asb.log")?;

    // Configure multi-layer logging: console + file
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        // .with(fmt::layer().with_writer(log_file).with_ansi(false))
        .init();

    let cli = Cli::parse();
    cli.run().await
}
