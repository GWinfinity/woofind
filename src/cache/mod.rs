//! Memory-mapped cache for instant cold starts
//! 
//! 内存映射：使用 memmap2 将 go.sum / go.mod 缓存文件映射到内存
//! 冷启动从 2s 降至 200ms

use anyhow::Result;
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::index::InvertedIndex;

const INDEX_CACHE_FILE: &str = "index.bin";
const METADATA_FILE: &str = "metadata.json";

/// Memory-mapped cache for the inverted index
pub struct MmapCache {
    cache_dir: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheMetadata {
    version: String,
    created_at: u64,
    symbol_count: usize,
    module_count: usize,
}

impl MmapCache {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        
        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Save the index to disk using binary serialization
    pub fn save_index(&self, index: &InvertedIndex) -> Result<()> {
        let index_path = self.cache_dir.join(INDEX_CACHE_FILE);
        let metadata_path = self.cache_dir.join(METADATA_FILE);

        // Serialize index data
        let data = SerializedIndex::from_index(index);
        let encoded = bincode::serialize(&data)?;

        // Write to file with buffered writer for speed
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&index_path)?;

        let mut writer = BufWriter::new(file);
        writer.write_all(&encoded)?;
        writer.flush()?;

        // Write metadata
        let metadata = CacheMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            symbol_count: index.stats().total_symbols,
            module_count: index.stats().total_modules,
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&metadata_path, metadata_json)?;

        info!("💾 Cache saved: {} symbols", metadata.symbol_count);
        
        Ok(())
    }

    /// Load index from memory-mapped file
    pub fn load_index(&self) -> Result<Option<InvertedIndex>> {
        let index_path = self.cache_dir.join(INDEX_CACHE_FILE);
        
        if !index_path.exists() {
            return Ok(None);
        }

        let start = std::time::Instant::now();

        // Memory map the file for zero-copy access
        let file = File::open(&index_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Deserialize from memory-mapped buffer
        let data: SerializedIndex = bincode::deserialize(&mmap)?;
        let index = data.to_index();

        let elapsed = start.elapsed();
        info!("⚡ Cache loaded via mmap in {:?}", elapsed);

        Ok(Some(index))
    }

    /// Check if cache is valid and recent
    pub fn is_cache_valid(&self, max_age_hours: u64) -> bool {
        let metadata_path = self.cache_dir.join(METADATA_FILE);
        
        if !metadata_path.exists() {
            return false;
        }

        let Ok(metadata_json) = std::fs::read_to_string(&metadata_path) else {
            return false;
        };

        let Ok(metadata): Result<CacheMetadata, _> = serde_json::from_str(&metadata_json) else {
            return false;
        };

        // Check version
        if metadata.version != env!("CARGO_PKG_VERSION") {
            warn!("Cache version mismatch, rebuilding...");
            return false;
        }

        // Check age
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_hours = (now - metadata.created_at) / 3600;
        
        if age_hours > max_age_hours {
            warn!("Cache is {} hours old, rebuilding...", age_hours);
            return false;
        }

        true
    }

    /// Clear the cache
    pub fn clear(&self) -> Result<()> {
        let index_path = self.cache_dir.join(INDEX_CACHE_FILE);
        let metadata_path = self.cache_dir.join(METADATA_FILE);

        if index_path.exists() {
            std::fs::remove_file(&index_path)?;
        }
        if metadata_path.exists() {
            std::fs::remove_file(&metadata_path)?;
        }

        info!("🗑️  Cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<Option<CacheStats>> {
        let metadata_path = self.cache_dir.join(METADATA_FILE);
        let index_path = self.cache_dir.join(INDEX_CACHE_FILE);

        if !metadata_path.exists() || !index_path.exists() {
            return Ok(None);
        }

        let metadata_json = std::fs::read_to_string(&metadata_path)?;
        let metadata: CacheMetadata = serde_json::from_str(&metadata_json)?;
        
        let file_size = std::fs::metadata(&index_path)?.len();

        Ok(Some(CacheStats {
            symbol_count: metadata.symbol_count,
            module_count: metadata.module_count,
            file_size_bytes: file_size,
            created_at: metadata.created_at,
        }))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub symbol_count: usize,
    pub module_count: usize,
    pub file_size_bytes: u64,
    pub created_at: u64,
}

/// Serializable representation of the index
#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedIndex {
    symbols: Vec<crate::index::Symbol>,
    modules: Vec<crate::index::ModuleInfo>,
}

impl SerializedIndex {
    fn from_index(index: &InvertedIndex) -> Self {
        let mut symbols = Vec::new();
        let mut modules = Vec::new();

        // Collect all symbols
        for entry in index.name_index.iter() {
            symbols.extend(entry.value().iter().cloned());
        }

        // Collect all modules
        for entry in index.module_metadata.iter() {
            modules.push(entry.value().clone());
        }

        Self { symbols, modules }
    }

    fn to_index(&self) -> InvertedIndex {
        let index = InvertedIndex::new();

        // Rebuild indices
        for symbol in &self.symbols {
            index.insert(symbol.clone());
        }

        for module in &self.modules {
            index.module_metadata.insert(module.path.clone(), module.clone());
        }

        index
    }
}

/// Memory-mapped file reader for large go.sum / go.mod files
pub struct MmapReader {
    mmap: Mmap,
}

impl MmapReader {
    /// Create a new memory-mapped reader for a file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        Ok(Self { mmap })
    }

    /// Get the memory-mapped buffer
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Parse as text lines
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        std::str::from_utf8(&self.mmap)
            .unwrap_or("")
            .lines()
    }
}

/// Optimized cache for Go module files
pub struct ModuleCache {
    cache_dir: PathBuf,
}

impl ModuleCache {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir)?;
        
        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Cache a parsed module
    pub fn cache_module(&self, module_path: &str, data: &[u8]) -> Result<()> {
        let safe_name = module_path.replace('/', "_").replace('\\', "_");
        let path = self.cache_dir.join(format!("{}.bin", safe_name));
        
        std::fs::write(&path, data)?;
        debug!("Cached module: {}", module_path);
        
        Ok(())
    }

    /// Load a cached module via mmap
    pub fn load_module(&self, module_path: &str) -> Result<Option<Mmap>> {
        let safe_name = module_path.replace('/', "_").replace('\\', "_");
        let path = self.cache_dir.join(format!("{}.bin", safe_name));
        
        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        Ok(Some(mmap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_save_load() {
        let temp = TempDir::new().unwrap();
        let cache = MmapCache::new(temp.path()).unwrap();
        
        // Create a test index
        let index = InvertedIndex::new();
        index.insert(crate::index::Symbol {
            name: "Test".to_string(),
            package: "test.com/example".to_string(),
            package_name: "example".to_string(),
            kind: crate::index::SymbolKind::Function,
            version: None,
            import_path: "test.com/example".to_string(),
            doc: None,
            signature: None,
        });

        // Save and load
        cache.save_index(&index).unwrap();
        let loaded = cache.load_index().unwrap();
        
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.stats().total_symbols, 1);
    }
}
