//! Query Engine - Fuzzy matching and symbol resolution
//!
//! 模糊匹配（Fuzzy Finding）如 redis.NewClient → github.com/redis/go-redis
//! 微秒级实时解析（Real-time Resolution）

#![allow(dead_code)]

use super::{InvertedIndex, Symbol};
use dashmap::DashMap;
use nucleo::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo::{Matcher, Utf32String};
use std::sync::Arc;
use tracing::debug;

/// Query engine for searching the index
pub struct QueryEngine {
    index: Arc<InvertedIndex>,
    fuzzy_matcher: FuzzyMatcher,
}

impl QueryEngine {
    pub fn new(index: Arc<InvertedIndex>) -> Self {
        Self {
            index: Arc::clone(&index),
            fuzzy_matcher: FuzzyMatcher::new(index),
        }
    }

    /// Exact lookup by symbol name (microsecond-level)
    pub fn exact_lookup(&self, name: &str) -> Vec<Symbol> {
        let start = std::time::Instant::now();

        let result = self.index.get_by_name(name).unwrap_or_default();

        let elapsed = start.elapsed();
        debug!("Exact lookup for '{}' took {:?}", name, elapsed);

        result
    }

    /// Lookup with package prefix (e.g., "redis.NewClient")
    pub fn qualified_lookup(&self, qualified_name: &str) -> Vec<Symbol> {
        let start = std::time::Instant::now();

        let parts: Vec<&str> = qualified_name.split('.').collect();
        let result = match parts.len() {
            1 => self.exact_lookup(parts[0]),
            2 => {
                let package_hint = parts[0];
                let symbol_name = parts[1];

                self.index
                    .get_by_name(symbol_name)
                    .map(|symbols| {
                        symbols
                            .into_iter()
                            .filter(|s| {
                                s.package_name == package_hint || s.package.contains(package_hint)
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            _ => vec![],
        };

        let elapsed = start.elapsed();
        debug!(
            "Qualified lookup for '{}' took {:?}",
            qualified_name, elapsed
        );

        result
    }

    /// Fuzzy search for symbols
    pub fn fuzzy_search(&self, query: &str, limit: usize) -> Vec<(Symbol, u32)> {
        self.fuzzy_matcher.search(query, limit)
    }

    /// Smart search: tries exact, then qualified, then fuzzy
    pub fn smart_search(&self, query: &str, limit: usize) -> Vec<Symbol> {
        // 1. Try exact match
        let exact = self.exact_lookup(query);
        if !exact.is_empty() {
            return exact;
        }

        // 2. Try qualified match (package.symbol)
        if query.contains('.') {
            let qualified = self.qualified_lookup(query);
            if !qualified.is_empty() {
                return qualified;
            }
        }

        // 3. Fall back to fuzzy search
        self.fuzzy_search(query, limit)
            .into_iter()
            .map(|(symbol, _)| symbol)
            .collect()
    }

    /// Get auto-completion suggestions
    pub fn autocomplete(&self, prefix: &str, limit: usize) -> Vec<String> {
        if prefix.is_empty() {
            return vec![];
        }

        let first_char = prefix.chars().next().unwrap();

        self.index
            .prefix_index
            .get(&first_char)
            .map(|entry| {
                entry
                    .iter()
                    .filter(|name| name.starts_with(prefix))
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// High-performance fuzzy matcher using nucleo
pub struct FuzzyMatcher {
    index: Arc<InvertedIndex>,
    matcher: Mutex<Matcher>,
    // Cache for pre-converted strings
    string_cache: Arc<DashMap<String, Utf32String>>,
}

use std::sync::Mutex;

impl FuzzyMatcher {
    pub fn new(index: Arc<InvertedIndex>) -> Self {
        Self {
            index,
            matcher: Mutex::new(Matcher::new(nucleo::Config::DEFAULT)),
            string_cache: Arc::new(DashMap::new()),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(Symbol, u32)> {
        let start = std::time::Instant::now();

        // Get all symbol names (this could be optimized with a pre-built FST)
        let all_names = self.index.all_symbol_names();

        // Create pattern atom for nucleo
        let atom = Atom::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
            false,
        );

        let mut matcher = self.matcher.lock().unwrap();
        let mut results: Vec<(Symbol, u32)> = Vec::with_capacity(limit);

        for name in all_names {
            // Convert to Utf32String (nucleo's internal format)
            let utf32 = self.get_or_convert(&name);

            if let Some(score) = atom.score(utf32.slice(..), &mut matcher) {
                if let Some(symbols) = self.index.get_by_name(&name) {
                    for symbol in symbols {
                        results.push((symbol, score as u32));
                    }
                }
            }

            if results.len() >= limit * 3 {
                break;
            }
        }

        // Sort by score (higher is better for nucleo)
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(limit);

        let elapsed = start.elapsed();
        debug!("Fuzzy search for '{}' took {:?}", query, elapsed);

        results
    }

    fn get_or_convert(&self, s: &str) -> Utf32String {
        if let Some(cached) = self.string_cache.get(s) {
            return cached.clone();
        }

        let utf32 = Utf32String::from(s);
        self.string_cache.insert(s.to_string(), utf32.clone());
        utf32
    }
}

/// Simple fuzzy matcher for fallback
pub struct SimpleFuzzyMatcher;

impl SimpleFuzzyMatcher {
    /// Calculate edit distance between two strings
    #[allow(clippy::needless_range_loop)]
    pub fn edit_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let len_a = a_chars.len();
        let len_b = b_chars.len();

        if len_a == 0 {
            return len_b;
        }
        if len_b == 0 {
            return len_a;
        }

        let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

        for i in 0..=len_a {
            matrix[i][0] = i;
        }
        for j in 0..=len_b {
            matrix[0][j] = j;
        }

        for i in 1..=len_a {
            for j in 1..=len_b {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len_a][len_b]
    }

    /// Check if query is a subsequence of target
    pub fn is_subsequence(query: &str, target: &str) -> bool {
        let mut query_chars = query.chars();
        let mut current = query_chars.next();

        for target_char in target.chars() {
            if let Some(q) = current {
                if q.to_lowercase().eq(target_char.to_lowercase()) {
                    current = query_chars.next();
                }
            } else {
                break;
            }
        }

        current.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SymbolKind;

    fn create_test_index() -> Arc<InvertedIndex> {
        let index = Arc::new(InvertedIndex::new());

        let symbols = vec![
            Symbol {
                name: "NewClient".to_string(),
                package: "github.com/redis/go-redis/v9".to_string(),
                package_name: "redis".to_string(),
                kind: SymbolKind::Function,
                version: Some("v9.5.1".to_string()),
                import_path: "github.com/redis/go-redis/v9".to_string(),
                doc: None,
                signature: None,
            },
            Symbol {
                name: "NewClient".to_string(),
                package: "github.com/go-redis/redis/v8".to_string(),
                package_name: "redis".to_string(),
                kind: SymbolKind::Function,
                version: Some("v8.11.5".to_string()),
                import_path: "github.com/go-redis/redis/v8".to_string(),
                doc: None,
                signature: None,
            },
            Symbol {
                name: "HandleFunc".to_string(),
                package: "net/http".to_string(),
                package_name: "http".to_string(),
                kind: SymbolKind::Function,
                version: None,
                import_path: "net/http".to_string(),
                doc: None,
                signature: None,
            },
        ];

        for symbol in symbols {
            index.insert(symbol);
        }

        index
    }

    #[test]
    fn test_exact_lookup() {
        let index = create_test_index();
        let engine = QueryEngine::new(index);

        let results = engine.exact_lookup("NewClient");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_qualified_lookup() {
        let index = create_test_index();
        let engine = QueryEngine::new(index);

        let results = engine.qualified_lookup("redis.NewClient");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_subsequence_match() {
        assert!(SimpleFuzzyMatcher::is_subsequence("ncl", "NewClient"));
        assert!(SimpleFuzzyMatcher::is_subsequence("hdl", "HandleFunc"));
        assert!(!SimpleFuzzyMatcher::is_subsequence("xyz", "NewClient"));
    }
}
