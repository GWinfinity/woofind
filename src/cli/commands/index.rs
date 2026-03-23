//! Index command - Build and maintain the symbol index

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;


use crate::cache::MmapCache;
use crate::cli::{print_info, print_success};
use crate::index::{IndexBuilder, InvertedIndex};

pub async fn run(path: &str, watch: bool) -> Result<()> {
    let root = Path::new(path);
    
    if !root.exists() {
        anyhow::bail!("Path '{}' does not exist", path);
    }

    print_info(&format!("Building index from: {}", path));

    // Create or load index
    let index = Arc::new(InvertedIndex::new());
    
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));
    
    let cache = Arc::new(MmapCache::new(&cache_dir)?);
    
    // Try to load from cache first
    let mut from_cache = false;
    if !watch {
        if let Ok(Some(cached_index)) = cache.load_index() {
            // Check if cache is reasonably fresh
            if cache.is_cache_valid(24) {
                print_success("Loaded index from cache");
                from_cache = true;
                
                // Use cached index
                let builder = IndexBuilder::with_index(Arc::new(cached_index))?;
                
                if watch {
                    print_info("Starting file watcher...");
                    let mut builder = builder;
                    builder.start_watching(root)?;
                    
                    // Keep running
                    tokio::signal::ctrl_c().await?;
                }
                
                return Ok(());
            }
        }
    }

    // Build from scratch
    let mut builder = IndexBuilder::with_index(index)?;
    
    let start = std::time::Instant::now();
    builder.build_from_directory(root)?;
    let elapsed = start.elapsed();

    let stats = builder.index().stats();
    print_success(&format!(
        "Indexed {} symbols from {} packages in {:?}",
        stats.total_symbols, stats.total_packages, elapsed
    ));

    // Save to cache
    if !from_cache {
        builder.save_to_cache()?;
    }

    // Start watching if requested
    if watch {
        print_info("Starting file watcher...");
        builder.start_watching(root)?;
        
        // Keep running until interrupted
        tokio::signal::ctrl_c().await?;
        print_info("Shutting down...");
    }

    Ok(())
}
