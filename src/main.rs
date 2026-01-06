//! Kaiak - Goose AI agent orchestrator for code migration workflows.

use anyhow::Result;
use kaiak::cli::Cli;
use kaiak::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;

    let cli = Cli::parse_args();
    cli.run().await
}
