//! woofind - Blazing-fast Go import discovery
//!
//! 零成本抽象：DashMap 提供 Go 的 sync.Map 无法比拟的无锁并发读性能
//! 内存映射：使用 memmap2 将缓存文件映射到内存，冷启动从 2s 降至 200ms
//! 增量更新：基于 notify 库监听文件系统事件，变更时仅更新差量索引

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod api;
mod cache;
mod cli;
mod index;
mod parser;

use cli::commands;

#[derive(Parser)]
#[command(
    name = "woofind",
    about = "🔍 Blazing-fast Go import discovery",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Index Go modules in the given directory
    Index {
        /// Directory to scan for Go modules
        #[arg(default_value = ".")]
        path: String,

        /// Watch for file changes and update index incrementally
        #[arg(short, long)]
        watch: bool,
    },

    /// Query the index for a symbol
    Query {
        /// Symbol to search for (e.g., "redis.NewClient")
        symbol: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Enable fuzzy matching
        #[arg(short, long)]
        fuzzy: bool,
    },

    /// Start the HTTP API server
    Serve {
        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1:7373")]
        bind: String,
    },

    /// Show index statistics
    Stats,

    /// Clear the cache
    Clear,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| if cli.verbose { "debug" } else { "info" }.into()),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("🐕 woofind starting...");

    match cli.command {
        Commands::Index { path, watch } => {
            commands::index::run(&path, watch).await?;
        }
        Commands::Query {
            symbol,
            limit,
            fuzzy,
        } => {
            commands::query::run(&symbol, limit, fuzzy).await?;
        }
        Commands::Serve { bind } => {
            commands::serve::run(&bind).await?;
        }
        Commands::Stats => {
            commands::stats::run().await?;
        }
        Commands::Clear => {
            commands::clear::run().await?;
        }
    }

    Ok(())
}
