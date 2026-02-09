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
│   ├── aapt2.rs          # aapt2 wrapper with parallel support
│   ├── aar.rs            # AAR extraction
│   ├── builder.rs        # Main build orchestration
│   ├── cache.rs          # Incremental build cache
│   ├── cli.rs            # CLI interface
│   ├── dependency.rs     # Multi-app dependency resolution
│   ├── resource_priority.rs # Resource overlay and priority handling
│   ├── merge.rs          # Internal package merging utilities
│   ├── types.rs          # Type definitions
│   └── main.rs           # Entry point
├── examples/
│   ├── simple-skin/      # Example skin project
│   └── multi-theme/      # Multi-theme example
├── target/
│   └── release/
│       └── asb           # Compiled binary
└── Cargo.toml            # Rust dependencies
```

## Making Changes

1. Create a new branch for your feature or bugfix
2. Write clean, readable Rust code
3. Follow the existing code style
4. Update documentation if needed
5. Test your changes thoroughly

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
