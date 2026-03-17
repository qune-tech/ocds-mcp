use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use ocds_mcp::embedder::SentenceEmbedder;
use ocds_mcp::server::OcdsMcpServer;
use ocds_mcp::state::SharedState;
use rmcp::{ServiceExt, transport::stdio};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "ocds-mcp", about = "MCP server for German public procurement data (Vergabe Dashboard)")]
struct Cli {
    /// Path to the local SQLite database (for company profiles)
    #[arg(long, default_value = "profiles.db")]
    db: String,

    /// Directory for local data files
    #[arg(long, default_value = "data")]
    data_dir: String,

    /// URL of the Vergabe Dashboard API
    #[arg(long, default_value = "https://vergabe-dashboard.qune.de")]
    api_url: String,

    /// API key for authenticating with the API (also reads OCDS_API_KEY env var)
    #[arg(long, env = "OCDS_API_KEY")]
    api_key: Option<String>,
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ort=warn"));

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .with_ansi(false)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging();

    // Ensure data directory exists
    std::fs::create_dir_all(&cli.data_dir)?;

    // Open or create the local database (company profiles)
    info!("Opening profile database: {}", cli.db);
    let db = ocds_mcp::profile_db::ProfileDb::open(&cli.db)?;

    let http = reqwest::Client::new();
    info!("REST API URL: {}", cli.api_url);

    if cli.api_key.is_some() {
        info!("API key configured for REST API authentication");
    }

    let state = Arc::new(SharedState {
        db: std::sync::Mutex::new(db),
        data_dir: cli.data_dir,
        embedder: std::sync::OnceLock::new(),
        api_url: cli.api_url,
        http,
        api_key: cli.api_key,
    });

    // Load embedder in background — MCP stdio starts immediately
    let state_bg = Arc::clone(&state);
    tokio::spawn(async move {
        match SentenceEmbedder::new().await {
            Ok(e) => {
                let _ = state_bg.embedder.set(e);
                info!("Embedder loaded successfully (multilingual-e5-small, 384-dim)");
            }
            Err(e) => {
                info!("Embedder not available: {e}. Search and embedding features will be disabled.");
            }
        }
    });

    let server = OcdsMcpServer::new(state);
    info!("Serving MCP over stdio");
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    info!("MCP server shutdown");
    Ok(())
}
