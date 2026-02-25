# Copilot Instructions for ASB (Android Skin Builder)

## Project Summary

ASB is a high-performance Rust CLI tool for packaging Android resource-only skin packages using `aapt2`. It wraps the Android `aapt2` binary to compile and link resource files into `.skin` packages, supporting incremental builds, parallel compilation, AAR dependencies, multi-app configurations, flavors, and stable resource IDs.

- **Language**: Rust (edition 2021, requires Rust 1.70+)
- **Type**: CLI tool with a library interface (`lib` + `bin` targets)
- **Key external dependency at runtime**: `aapt2` from the Android SDK

## Build, Test, and Validation Commands

Always run these commands from the repository root.

```bash
# Build (release profile with LTO and strip)
cargo build --release

# Run unit and integration tests
cargo test

# Format code (always run before committing)
cargo fmt

# Lint (fix warnings before submitting)
cargo clippy
```

No additional bootstrap steps are required. The `Cargo.toml` pins all dependencies; simply running `cargo build` will fetch and compile them.

## Project Layout

```
asb/
├── .github/
│   ├── copilot-instructions.md   # This file
│   └── workflows/
│       ├── build.yml             # CI: multi-platform build + binary smoke test
│       └── release.yml           # CD: triggered on v*.*.* tags, creates GitHub release
├── src/
│   ├── main.rs                   # Binary entry point (delegates to cli.rs)
│   ├── lib.rs                    # Library interface (re-exports modules)
│   ├── cli.rs                    # Clap-based CLI definitions and dispatch
│   ├── builder.rs                # Core build orchestration
│   ├── aapt2.rs                  # aapt2 subprocess wrapper, parallel compile/link
│   ├── aar.rs                    # AAR file extraction
│   ├── cache.rs                  # Incremental build cache using SHA-256
│   ├── dependency.rs             # Multi-app dependency resolution
│   ├── resource_priority.rs      # Android resource priority/overlay handling
│   ├── merge.rs                  # Internal package merging utilities
│   └── types.rs                  # Shared type definitions (config structs, errors)
├── tests/
│   └── integration_test.rs       # Integration tests
├── examples/                     # Example skin projects (not compiled as tests)
├── Cargo.toml                    # Rust manifest; release profile: lto=true, strip=true
├── README.md
└── CONTRIBUTING.md
```

## Key Architectural Notes

- **Config resolution order**: CLI args (highest) → `--config` file → `./asb.config.json`
- **Config formats**: Single-app (`SkinConfig`) and multi-app (`MultiSkinConfig` with `apps` array). Both are defined in `src/types.rs`.
- **Concurrency**: resource compilation uses Rayon (thread pool); multi-app builds use Tokio async tasks.
- **Incremental builds**: SHA-256 hashes of source files are stored in `buildDir` (default `{outputDir}/.build`). The deprecated `cacheDir` field maps to the same location.
- **Resource priority** (lowest → highest): AAR dependencies → `resourceDir` → `additionalResourceDirs`. Implemented via aapt2's `-R` overlay flag.
- **Package ID**: defaults to `0x7f`; configurable via `packageId` field or `--package-id` flag.
- **AndroidManifest.xml** is optional; a minimal one is auto-generated when omitted.
- **`androidJar`** is optional; auto-detected from `$ANDROID_HOME/platforms/` (highest API level).
- **Environment variables** in config paths (e.g., `${ANDROID_HOME}`) are expanded via `shellexpand`.

## CI/CD Workflows

### `build.yml` — runs on push/PR to `main`/`master`

Builds for all targets and then smoke-tests the binary with `--version`:

| Target | Runner | Notes |
|--------|--------|-------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | Standard Linux |
| `aarch64-unknown-linux-gnu` | ubuntu-latest | Requires `gcc-aarch64-linux-gnu` + `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc` |
| `x86_64-unknown-linux-gnu` | ubuntu-latest (container: `debian:11`) | glibc 2.31 compatibility build |
| `x86_64-apple-darwin` / `aarch64-apple-darwin` | macos-latest | |
| `x86_64-pc-windows-msvc` / `aarch64-pc-windows-msvc` | windows-latest | |

### `release.yml` — triggered by `v*.*.*` tags

Same matrix as build, plus creates `.tar.gz`/`.zip` archives and a GitHub release.

## Code Style

- Follow idiomatic Rust; run `cargo fmt` before every commit.
- Run `cargo clippy` and resolve all warnings before submitting a PR.
- Error handling: use `anyhow` for application errors and `thiserror` for library error types defined in `types.rs`.
- Logging: use the `tracing` crate (not `println!` or `eprintln!`).
- Keep functions focused; add comments only for non-obvious logic.
