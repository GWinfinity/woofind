use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use woofind::index::{InvertedIndex, QueryEngine, Symbol, SymbolKind};

fn create_test_index() -> Arc<InvertedIndex> {
    let index = Arc::new(InvertedIndex::new());
    
    // 插入测试符号
    for pkg in 0..100 {
        for sym in 0..100 {
            let symbol = Symbol {
                name: format!("Symbol{}", sym),
                package: format!("github.com/example/pkg{}", pkg),
                package_name: format!("pkg{}", pkg),
                kind: SymbolKind::Function,
                version: Some("v1.0.0".to_string()),
                import_path: format!("github.com/example/pkg{}", pkg),
                doc: Some("Test documentation".to_string()),
                signature: Some("func()".to_string()),
            };
            index.insert(symbol);
        }
    }
    
    index
}

fn exact_query_benchmark(c: &mut Criterion) {
    let index = create_test_index();
    let engine = QueryEngine::new(index);
    
    c.bench_function("exact_query", |b| {
        b.iter(|| {
            let result = engine.exact_lookup(black_box("Symbol50"));
            black_box(result);
        })
    });
}

fn fuzzy_query_benchmark(c: &mut Criterion) {
    let index = create_test_index();
    let engine = QueryEngine::new(index);
    
    c.bench_function("fuzzy_query", |b| {
        b.iter(|| {
            let result = engine.fuzzy_search(black_box("Smb50"), 10);
            black_box(result);
        })
    });
}

fn smart_search_benchmark(c: &mut Criterion) {
    let index = create_test_index();
    let engine = QueryEngine::new(index);
    
    c.bench_function("smart_search", |b| {
        b.iter(|| {
            let result = engine.smart_search(black_box("pkg1.Symbol5"), 10);
            black_box(result);
        })
    });
}

fn concurrent_query_benchmark(c: &mut Criterion) {
    use rayon::prelude::*;
    
    let index = create_test_index();
    
    c.bench_function("concurrent_reads_16_threads", |b| {
        b.iter(|| {
            (0..16).into_par_iter().for_each(|i| {
                let engine = QueryEngine::new(Arc::clone(&index));
                let result = engine.exact_lookup(&format!("Symbol{}", i));
                black_box(result);
            });
        })
    });
}

criterion_group!(
    benches,
    exact_query_benchmark,
    fuzzy_query_benchmark,
    smart_search_benchmark,
    concurrent_query_benchmark
);
criterion_main!(benches);
