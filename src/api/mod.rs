//! HTTP API for real-time symbol resolution
//! 
//! 微秒级实时解析（Real-time Resolution）

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::index::{QueryEngine, Symbol};

/// API state shared across handlers
pub struct ApiState {
    pub engine: Arc<QueryEngine>,
}

/// Search request parameters
#[derive(Deserialize)]
pub struct SearchRequest {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub fuzzy: bool,
}

fn default_limit() -> usize {
    10
}

/// Search response
#[derive(Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SymbolDto>,
    pub count: usize,
    pub elapsed_us: u64,
}

/// Symbol DTO for API responses
#[derive(Serialize)]
pub struct SymbolDto {
    pub name: String,
    pub package: String,
    pub package_name: String,
    pub kind: String,
    pub import_path: String,
    pub signature: Option<String>,
    pub doc: Option<String>,
}

impl From<Symbol> for SymbolDto {
    fn from(s: Symbol) -> Self {
        Self {
            name: s.name,
            package: s.package,
            package_name: s.package_name,
            kind: s.kind.to_string(),
            import_path: s.import_path,
            signature: s.signature,
            doc: s.doc,
        }
    }
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Create the API router
pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/search", get(search_handler))
        .with_state(state)
}

async fn health_handler(State(_state): State<Arc<ApiState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // Would track actual uptime in production
    })
}

async fn search_handler(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let start = std::time::Instant::now();

    let results = if params.fuzzy {
        state
            .engine
            .fuzzy_search(&params.q, params.limit)
            .into_iter()
            .map(|(s, _)| s)
            .collect()
    } else {
        state.engine.smart_search(&params.q, params.limit)
    };

    let elapsed = start.elapsed();

    let dtos: Vec<SymbolDto> = results.into_iter().map(Into::into).collect();

    Ok(Json(SearchResponse {
        query: params.q,
        count: dtos.len(),
        results: dtos,
        elapsed_us: elapsed.as_micros() as u64,
    }))
}
