# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- FST (Finite State Transducer) for prefix matching
- WebSocket support for real-time updates
- LSP (Language Server Protocol) implementation
- Integration with wootype for type-aware symbol resolution
- Integration with woofmt for import optimization

## [0.1.0] - 2026-03-23

### Added
- Initial release of woofind
- **Inverted Index**: DashMap-based lock-free concurrent symbol index
- **Memory-mapped Cache**: memmap2 for 200ms cold start (vs 2s traditional)
- **Incremental Updates**: notify-based file system watching
- **Fuzzy Matching**: nucleo engine for approximate symbol matching
- **HTTP API**: Axum-based REST API for real-time queries
- **CLI Commands**:
  - `index`: Build and update symbol index
  - `query`: Search symbols (exact/fuzzy)
  - `serve`: Start HTTP API server
  - `stats`: Show index statistics
  - `clear`: Clear cache

### Performance
- Exact query: ~40-150μs (10x+ faster than traditional approaches)
- Fuzzy search: ~80-320μs
- Hot start: ~3-7ms via mmap
- Concurrent reads: 16 threads with near-linear scaling
- Binary size: 3.1MB single file

### Documentation
- README with usage examples
- PERFORMANCE.md with benchmark results
- ECOSYSTEM.md describing Woo ecosystem integration
- CONTRIBUTING.md with development guidelines

### Infrastructure
- Git repository with main branch
- GitHub Actions CI/CD (Linux, macOS)
- Pre-commit hooks for code quality
- Apache-2.0 License

[Unreleased]: https://github.com/GWinfinity/woofind/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/GWinfinity/woofind/releases/tag/v0.1.0
