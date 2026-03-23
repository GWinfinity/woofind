//! Query command - Search for Go symbols

use anyhow::Result;
use colored::Colorize;
use std::sync::Arc;

use crate::cache::MmapCache;
use crate::cli::{print_error, print_info};
use crate::index::QueryEngine;

pub async fn run(symbol: &str, limit: usize, fuzzy: bool) -> Result<()> {
    // Load index from cache
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));

    let cache = MmapCache::new(&cache_dir)?;

    let index = match cache.load_index()? {
        Some(idx) => Arc::new(idx),
        None => {
            print_error("No index found. Run 'woofind index' first.");
            anyhow::bail!("Index not found");
        }
    };

    let engine = QueryEngine::new(index);

    let start = std::time::Instant::now();

    let results = if fuzzy {
        engine
            .fuzzy_search(symbol, limit)
            .into_iter()
            .map(|(s, _)| s)
            .collect()
    } else {
        engine.smart_search(symbol, limit)
    };

    let elapsed = start.elapsed();

    if results.is_empty() {
        print_info(&format!("No results found for '{}'", symbol));

        // Suggest fuzzy search if exact match failed
        if !fuzzy {
            println!("\n💡 Try with --fuzzy flag for approximate matching");
        }
    } else {
        println!(
            "\n🔍 Found {} result(s) for '{}' in {:?}\n",
            results.len().to_string().cyan(),
            symbol.yellow(),
            elapsed
        );

        for (i, sym) in results.iter().enumerate() {
            println!(
                "  {}. {} {} {}",
                (i + 1).to_string().dimmed(),
                sym.name.green().bold(),
                format!("({})", sym.kind).blue(),
                sym.signature
                    .as_ref()
                    .map(|s| format!("\n     {}", s.dimmed()))
                    .unwrap_or_default()
            );
            println!("     {}", sym.package.cyan());
            if let Some(doc) = &sym.doc {
                let preview: String = doc.chars().take(80).collect();
                println!("     {}", preview.dimmed());
            }
            println!();
        }

        // Show the top result import path
        if let Some(first) = results.first() {
            println!("📦 Quick import:");
            println!("   {}", format!("import \"{}\"", first.import_path).green());
            println!();
        }
    }

    Ok(())
}
