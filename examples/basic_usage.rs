//! Basic usage example for woofind library
//!
//! Run with: cargo run --example basic_usage

use std::path::Path;
use woofind::Woofind;

fn main() -> anyhow::Result<()> {
    // Create a new woofind client
    let client = Woofind::new();
    
    println!("woofind client created!");
    println!("Stats: {}", client.stats());
    
    // In a real scenario, you would load/build an index:
    // let client = Woofind::load_or_build(Path::new("/path/to/go/project"))?;
    
    // Search for symbols
    // let results = client.search("NewClient", 10);
    // for symbol in results {
    //     println!("Found: {} in {}", symbol.name, symbol.package);
    // }
    
    Ok(())
}
