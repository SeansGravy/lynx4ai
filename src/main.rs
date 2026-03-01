//! lynx4ai — Rust MCP server for AI browser automation.
//!
//! Communicates via stdio (MCP protocol). All logging goes to stderr.
//! NEVER use println!() — it corrupts the MCP JSON-RPC stream on stdout.

use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

mod auth;
mod browser;
mod error;
mod server;
mod snapshot;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing to STDERR only (critical for MCP stdio transport)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "lynx4ai=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("lynx4ai starting");

    let transport = rmcp::transport::io::stdio();
    let service = server::LynxServer::new().serve(transport).await?;

    service.waiting().await?;

    tracing::info!("lynx4ai shutting down");
    Ok(())
}
