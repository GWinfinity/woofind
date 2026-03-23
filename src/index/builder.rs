//! Index Builder - Scans Go modules and builds the inverted index
//!
//! 增量更新：基于 notify 库监听文件系统事件，变更时仅更新差量索引

use super::{InvertedIndex, ModuleInfo};
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::cache::MmapCache;
use crate::parser::{GoModuleParser, ParsedModule};

/// Builder for constructing and updating the inverted index
pub struct IndexBuilder {
    index: Arc<InvertedIndex>,
    cache: Arc<MmapCache>,
    parser: Arc<GoModuleParser>,
    watch_tx: Option<Sender<IndexEvent>>,
    watch_rx: Option<Receiver<IndexEvent>>,
}

#[derive(Debug, Clone)]
pub enum IndexEvent {
    FileChanged(PathBuf),
    FileRemoved(PathBuf),
    ModuleAdded(PathBuf),
    ModuleRemoved(String), // module path
}

impl IndexBuilder {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .map(|d| d.join("woofind"))
            .unwrap_or_else(|| PathBuf::from(".woofind_cache"));

        Ok(Self {
            index: Arc::new(InvertedIndex::new()),
            cache: Arc::new(MmapCache::new(&cache_dir)?),
            parser: Arc::new(GoModuleParser::new()),
            watch_tx: None,
            watch_rx: None,
        })
    }

    pub fn with_index(index: Arc<InvertedIndex>) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .map(|d| d.join("woofind"))
            .unwrap_or_else(|| PathBuf::from(".woofind_cache"));

        Ok(Self {
            index,
            cache: Arc::new(MmapCache::new(&cache_dir)?),
            parser: Arc::new(GoModuleParser::new()),
            watch_tx: None,
            watch_rx: None,
        })
    }

    /// Build index from scratch by scanning a directory
    pub fn build_from_directory(&self, root: &Path) -> Result<()> {
        let start = Instant::now();
        info!("🔍 Scanning for Go modules in {:?}...", root);

        // Find all go.mod files
        let go_mod_files: Vec<PathBuf> = WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name() == "go.mod" && !e.path().to_string_lossy().contains("vendor/")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        info!("📦 Found {} go.mod files", go_mod_files.len());

        // Parse modules in parallel using Rayon
        let modules: Vec<ParsedModule> = go_mod_files
            .par_iter()
            .filter_map(|path| match self.parser.parse_module(path) {
                Ok(module) => Some(module),
                Err(e) => {
                    warn!("Failed to parse {:?}: {}", path, e);
                    None
                }
            })
            .collect();

        info!("✅ Parsed {} modules", modules.len());

        // Build index in parallel
        modules.par_iter().for_each(|module| {
            self.add_module_to_index(module);
        });

        let elapsed = start.elapsed();
        info!("📊 Index built in {:?}", elapsed);
        info!("   {}", self.index.stats());

        Ok(())
    }

    /// Build index from mmap cache (fast cold start)
    pub fn build_from_cache(&self) -> Result<bool> {
        let start = Instant::now();
        info!("💾 Loading index from cache...");

        match self.cache.load_index()? {
            Some(cached_index) => {
                // Merge cached data into current index
                for entry in cached_index.name_index.iter() {
                    let (_name, symbols) = entry.pair();
                    for symbol in symbols {
                        self.index.insert(symbol.clone());
                    }
                }

                let elapsed = start.elapsed();
                info!("⚡ Cache loaded in {:?}", elapsed);
                info!("   {}", self.index.stats());
                Ok(true)
            }
            None => {
                info!("   No cache found, will build from scratch");
                Ok(false)
            }
        }
    }

    /// Save current index to cache
    pub fn save_to_cache(&self) -> Result<()> {
        info!("💾 Saving index to cache...");
        self.cache.save_index(&self.index)?;
        info!("   Cache saved successfully");
        Ok(())
    }

    /// Add a parsed module to the index
    fn add_module_to_index(&self, module: &ParsedModule) {
        let module_info = ModuleInfo {
            path: module.path.clone(),
            version: module.version.clone().unwrap_or_default(),
            go_version: module.go_version.clone(),
            symbol_count: module.symbols.len(),
        };

        self.index
            .module_metadata
            .insert(module.path.clone(), module_info);

        // Batch insert all symbols
        self.index.batch_insert(module.symbols.clone());
    }

    /// Start watching for file system changes (incremental updates)
    pub fn start_watching(&mut self, root: &Path) -> Result<()> {
        let (tx, _rx) = unbounded::<IndexEvent>();
        let (event_tx, event_rx) = unbounded::<IndexEvent>();
        self.watch_tx = Some(tx.clone());
        self.watch_rx = Some(_rx);

        let watcher_tx = event_tx.clone();
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        debug!("File system event: {:?}", event);

                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                for path in event.paths {
                                    if path.file_name() == Some(std::ffi::OsStr::new("go.mod")) {
                                        let _ = watcher_tx.send(IndexEvent::ModuleAdded(path));
                                    } else if path.extension() == Some(std::ffi::OsStr::new("go")) {
                                        let _ = watcher_tx.send(IndexEvent::FileChanged(path));
                                    }
                                }
                            }
                            EventKind::Remove(_) => {
                                for path in event.paths {
                                    if path.file_name() == Some(std::ffi::OsStr::new("go.mod")) {
                                        // Try to extract module path from parent directory
                                        if let Some(parent) = path.parent() {
                                            let _ = watcher_tx.send(IndexEvent::ModuleRemoved(
                                                parent.to_string_lossy().to_string(),
                                            ));
                                        }
                                    } else {
                                        let _ = watcher_tx.send(IndexEvent::FileRemoved(path));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => error!("Watch error: {}", e),
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;

        watcher.watch(root, RecursiveMode::Recursive)?;
        info!("👁️  Started watching {:?} for changes", root);

        // Spawn a thread to process events
        let index = Arc::clone(&self.index);
        let parser = Arc::clone(&self.parser);
        let cache = Arc::clone(&self.cache);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                Self::process_events(event_rx, index, parser, cache).await;
            });
        });

        // Keep watcher alive
        std::mem::forget(watcher);

        Ok(())
    }

    /// Process index events (incremental updates)
    async fn process_events(
        rx: Receiver<IndexEvent>,
        index: Arc<InvertedIndex>,
        parser: Arc<GoModuleParser>,
        _cache: Arc<MmapCache>,
    ) {
        while let Ok(event) = rx.recv() {
            let start = Instant::now();

            match event {
                IndexEvent::ModuleAdded(path) => {
                    info!("📦 New module detected: {:?}", path);
                    if let Ok(module) = parser.parse_module(&path) {
                        // Remove old entries for this module if any
                        index.remove_package(&module.path);

                        // Add new symbols
                        for symbol in &module.symbols {
                            index.insert(symbol.clone());
                        }

                        info!(
                            "   Added {} symbols in {:?}",
                            module.symbols.len(),
                            start.elapsed()
                        );
                    }
                }
                IndexEvent::ModuleRemoved(module_path) => {
                    info!("🗑️  Module removed: {}", module_path);
                    index.remove_package(&module_path);
                    info!("   Removed in {:?}", start.elapsed());
                }
                IndexEvent::FileChanged(path) => {
                    debug!("📝 File changed: {:?}", path);
                    // For individual file changes, we could do finer-grained updates
                    // For now, re-parse the entire module
                    if let Some(parent) = path.parent() {
                        let go_mod = parent.join("go.mod");
                        if go_mod.exists() {
                            if let Ok(module) = parser.parse_module(&go_mod) {
                                index.remove_package(&module.path);
                                for symbol in &module.symbols {
                                    index.insert(symbol.clone());
                                }
                            }
                        }
                    }
                }
                IndexEvent::FileRemoved(path) => {
                    debug!("🗑️  File removed: {:?}", path);
                    // Similar to FileChanged, we'd need fine-grained symbol tracking
                }
            }
        }
    }

    /// Get the underlying index
    pub fn index(&self) -> Arc<InvertedIndex> {
        Arc::clone(&self.index)
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new().expect("Failed to create IndexBuilder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_builder() {
        let temp = TempDir::new().unwrap();
        let builder = IndexBuilder::new().unwrap();

        // Create a dummy go.mod
        std::fs::write(
            temp.path().join("go.mod"),
            r#"module test.com/example

go 1.21
"#,
        )
        .unwrap();

        // This will fail because there's no actual Go code, but it shouldn't panic
        let _ = builder.build_from_directory(temp.path());
    }
}
