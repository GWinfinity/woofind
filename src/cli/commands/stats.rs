//! Stats command - Show index statistics

use anyhow::Result;
use colored::Colorize;

use crate::cache::MmapCache;
use crate::cli::{print_info, print_table};

pub async fn run() -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));

    let cache = MmapCache::new(&cache_dir)?;

    match cache.stats()? {
        Some(stats) => {
            println!("\n📊 {}", "Index Statistics".bold());
            println!();

            let rows = vec![
                vec!["Total Symbols".to_string(), stats.symbol_count.to_string()],
                vec!["Total Modules".to_string(), stats.module_count.to_string()],
                vec![
                    "Cache Size".to_string(),
                    format!("{:.2} MB", stats.file_size_bytes as f64 / 1_048_576.0),
                ],
                vec![
                    "Created".to_string(),
                    format!(
                        "{}",
                        chrono::DateTime::from_timestamp(stats.created_at as i64, 0)
                            .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    ),
                ],
            ];

            print_table(&["Metric", "Value"], &rows);
            println!();
        }
        None => {
            print_info("No index found. Run 'woofind index' to build one.");
        }
    }

    Ok(())
}
