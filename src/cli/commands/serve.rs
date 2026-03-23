//! Serve command - Start HTTP API server

use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::warn;

use crate::cache::MmapCache;
use crate::index::{QueryEngine, Symbol};
use crate::index::InvertedIndex;

#[derive(Clone)]
struct AppState {
    engine: Arc<QueryEngine>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    fuzzy: bool,
}

fn default_limit() -> usize {
    10
}

#[derive(Serialize)]
struct SearchResponse {
    query: String,
    results: Vec<SymbolResult>,
    count: usize,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct SymbolResult {
    name: String,
    package: String,
    package_name: String,
    kind: String,
    import_path: String,
    signature: Option<String>,
    doc: Option<String>,
    score: Option<u32>,
}

impl From<Symbol> for SymbolResult {
    fn from(s: Symbol) -> Self {
        Self {
            name: s.name,
            package: s.package,
            package_name: s.package_name,
            kind: s.kind.to_string(),
            import_path: s.import_path,
            signature: s.signature,
            doc: s.doc,
            score: None,
        }
    }
}

pub async fn run(bind: &str) -> Result<()> {
    // Load index
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("woofind"))
        .unwrap_or_else(|| std::path::PathBuf::from(".woofind_cache"));

    let cache = MmapCache::new(&cache_dir)?;

    let index = match cache.load_index()? {
        Some(idx) => Arc::new(idx),
        None => {
            warn!("No index found, building empty index");
            Arc::new(InvertedIndex::new())
        }
    };

    let engine = Arc::new(QueryEngine::new(index));
    let state = AppState { engine };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/search", get(search))
        .route("/api/v1/search", get(search))
        .route("/api/v1/complete", get(autocomplete))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = bind.parse()?;
    
    println!("🚀 woofind API server running at http://{}", addr);
    println!("\nEndpoints:");
    println!("  GET  /health           - Health check");
    println!("  GET  /search?q=QUERY   - Search symbols");
    println!("  GET  /api/v1/search?q=QUERY&limit=10&fuzzy=true");
    println!("  GET  /api/v1/complete?prefix=PREFIX");
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    "woofind API - Go import discovery service\n"
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "woofind",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Json<SearchResponse> {
    let start = std::time::Instant::now();

    let (results, scores): (Vec<Symbol>, Vec<Option<u32>>) = if params.fuzzy {
        let fuzzy_results = state.engine.fuzzy_search(&params.q, params.limit);
        let symbols: Vec<Symbol> = fuzzy_results.iter().map(|(s, _)| s.clone()).collect();
        let scores: Vec<Option<u32>> = fuzzy_results.iter().map(|(_, s)| Some(*s)).collect();
        (symbols, scores)
    } else {
        let symbols = state.engine.smart_search(&params.q, params.limit);
        let scores = vec![None; symbols.len()];
        (symbols, scores)
    };

    let elapsed = start.elapsed();

    let mut symbol_results: Vec<SymbolResult> = results.into_iter().map(Into::into).collect();
    
    // Attach scores
    for (result, score) in symbol_results.iter_mut().zip(scores) {
        result.score = score;
    }

    Json(SearchResponse {
        query: params.q.clone(),
        count: symbol_results.len(),
        results: symbol_results,
        elapsed_ms: elapsed.as_micros() as u64 / 1000,
    })
}

#[derive(Deserialize)]
struct AutocompleteQuery {
    prefix: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Serialize)]
struct AutocompleteResponse {
    prefix: String,
    suggestions: Vec<String>,
}

async fn autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteQuery>,
) -> Json<AutocompleteResponse> {
    let suggestions = state.engine.autocomplete(&params.prefix, params.limit);

    Json(AutocompleteResponse {
        prefix: params.prefix,
        suggestions,
    })
}
