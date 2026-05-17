#![allow(clippy::all, dead_code, unused_variables)]

mod aapt2;
mod aar;
mod builder;
mod cache;
mod cli;
mod dependency;
mod error;
mod resource_priority;
mod types;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI first to check for quiet mode
    let cli = Cli::parse();

    // Initialize logging - output to both console and file
    // In quiet mode, only show error level logs
    let log_level = if cli.quiet { "error" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // Build subscriber layers
    let console_layer = fmt::layer().with_writer(std::io::stdout).with_ansi(true);
    let subscriber = tracing_subscriber::registry().with(env_filter);

    // Add file layer if --log-file is specified
    if let Some(ref log_path) = cli.log_file {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::File::create(log_path) {
            Ok(log_file) => {
                let file_layer = fmt::layer().with_writer(log_file).with_ansi(false);
                subscriber.with(console_layer).with(file_layer).init();
                return cli.run().await;
            }
            Err(e) => {
                eprintln!(
                    "Warning: could not create log file '{}': {}",
                    log_path.display(),
                    e
                );
            }
        }
    }

    // Fallback: console only
    subscriber.with(console_layer).init();

    cli.run().await
}
