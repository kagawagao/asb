# Contributing to ASB

Thank you for your interest in contributing to Android Skin Builder!

## Development Setup

1. Fork and clone the repository
2. Install Rust (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Build the project: `cargo build --release`
4. Make your changes in the `src/` directory
5. Test your changes

## Project Structure

```
asb/
├── src/
│   ├── aapt2.rs               # aapt2 wrapper with parallel compile and overlay link
│   ├── aar.rs                 # AAR file extraction
│   ├── builder.rs             # Main build orchestration (SkinBuilder)
│   ├── cache.rs               # Incremental build cache (SHA-256)
│   ├── cli.rs                 # Clap CLI interface and build dispatch
│   ├── dependency.rs          # Multi-app dependency resolution (topological sort)
│   ├── resource_priority.rs   # Android resource overlay and priority handling
│   ├── merge.rs               # Skin package merging/extraction (library API)
│   ├── error.rs               # Library error types (thiserror)
│   ├── types.rs               # Config structs (BuildConfig, AppConfig, etc.)
│   ├── lib.rs                 # Library entry point
│   └── main.rs                # Binary entry point
├── tests/
│   ├── integration_test.rs    # Config loading and flavor integration tests
│   └── unit_tests.rs          # Additional unit tests
├── examples/                  # Example skin projects
├── Cargo.toml
└── README.md
```

## Making Changes

1. Create a new branch for your feature or bugfix
2. Write clean, readable Rust code
3. Follow the existing code style (`cargo fmt`)
4. Add tests for new functionality
5. Ensure `cargo build`, `cargo test`, and `cargo clippy` pass
6. Update documentation if needed

## Submitting Pull Requests

1. Push your changes to your fork
2. Create a pull request with a clear description
3. Link any relevant issues
4. Wait for review

## Code Style

- Use idiomatic Rust
- Follow Rust formatting conventions (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and concise
- Run `cargo clippy` for linting

## Testing

Before submitting a PR:

1. Build the project: `cargo build --release`
2. Format code: `cargo fmt`
3. Lint code: `cargo clippy`
4. Test the CLI with the example projects
5. Ensure no compilation errors or warnings

## Reporting Issues

When reporting issues, please include:

- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Environment information (OS, Rust version, Android SDK version, etc.)
- Relevant error messages or logs

## Questions?

Feel free to open an issue for questions or discussions.

Thank you for contributing!
