//! API server example
//!
//! Run with: cargo run --example api_server

use std::net::SocketAddr;
use std::sync::Arc;
use woofind::cache::MmapCache;
use woofind::index::{InvertedIndex, QueryEngine};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load index from cache
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));

    let cache = MmapCache::new(&cache_dir)?;

    let index = match cache.load_index()? {
        Some(idx) => Arc::new(idx),
        None => {
            println!("No cache found. Please run 'woofind index' first.");
            return Ok(());
        }
    };

    let engine = QueryEngine::new(index);

    // Example queries
    println!("Querying symbols...");
    
    let results = engine.smart_search("NewClient", 10);
    println!("Found {} results", results.len());
    
    for symbol in results {
        println!("  - {} ({})", symbol.name, symbol.package);
    }

    Ok(())
}
