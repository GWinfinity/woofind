//! Clear command - Clear the cache

use anyhow::Result;

use crate::cache::MmapCache;
use crate::cli::print_success;

pub async fn run() -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));

    let cache = MmapCache::new(&cache_dir)?;
    cache.clear()?;

    print_success("Cache cleared successfully");
    Ok(())
}
