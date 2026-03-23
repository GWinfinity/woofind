# Contributing to woofind

Thank you for your interest in contributing to woofind! 🐕

## Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/woofind.git
cd woofind

# Build the project
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Git Workflow

### Setup pre-commit hooks

```bash
# Configure git to use the hooks in .githooks
git config core.hooksPath .githooks
```

### Making Changes

1. Create a new branch for your feature
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and commit them
   ```bash
   git add .
   git commit -m "Add: brief description of your changes"
   ```

3. Push to your fork
   ```bash
   git push origin feature/your-feature-name
   ```

4. Create a Pull Request

## Commit Message Format

We follow conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `perf:` - Performance improvement
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Build/tooling changes

Examples:
```
feat: add fuzzy matching for import paths
fix: handle unicode symbols in index
perf: optimize DashMap shard count
docs: update API documentation
```

## Code Quality

Before submitting a PR, ensure:

- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `cargo test` passes
- [ ] Documentation is updated if needed
- [ ] Benchmarks show no regressions

## Testing

Run the test suite:

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# Benchmarks
cargo bench
```

## Performance

woofind is built for speed. When making changes:

1. Run benchmarks before and after
2. Ensure no regressions in hot paths
3. Document any trade-offs

## Questions?

Open an issue or reach out to the maintainers.

Happy coding! 🚀
