# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --release     # Release build (LTO + stripped, edition 2024)
cargo build               # Debug build
cargo test                # All 233 unit + integration tests
cargo test -p asb -- test_name   # Run a single test (use full test name)
cargo fmt                 # Format before committing
cargo clippy              # Lint (fix warnings before PR)
```

Requires Rust 1.70+. No other bootstrap steps — `Cargo.toml` pins all dependencies.

## Architecture

ASB wraps `aapt2` to compile and link Android resources into resource-only `.skin` packages (valid APK files with `resources.arsc` + compiled resources, no DEX). The binary at `src/main.rs` initializes tracing and delegates to `src/cli.rs`.

### Build pipeline (`src/builder.rs`)

1. **Collect resource dirs** — main `resourceDir`, AAR-extracted resources (Library priority), `additionalResourceDirs` (Additional priority). Each gets a `ResourcePriority` value: Library < Additional < Main.
2. **Compile** — `aapt2 compile` invoked in parallel via Rayon. Each file gets a `.flat` intermediate. Incremental mode skips unchanged files using SHA-256 hashes from `BuildCache` (`src/cache.rs`).
3. **Link** — `aapt2 link` merges base flat files with overlay flat files using `-R` for priority semantics. A minimal `AndroidManifest.xml` is auto-generated if none provided. Assets from `assetsDir` are included via `-A`.
4. **Finalize** — Clean up AAR temp dirs.

### Config system (`src/types.rs`)

Three config formats are detected at load time (`BuildConfig::load_configs()`):
- **Single object** — `BuildConfig` directly
- **Array** — `Vec<BuildConfig>`
- **Multi-app object** — `MultiAppConfig` with `apps` array and optional `flavors` per app; expanded into `Vec<BuildConfig>` by `MultiAppConfig::into_build_configs()`

**Field resolution order**: flavor → app → common (top-level multi-app fields like `outputDir`, `versionCode`, `assetsDir`). CLI args override all. When no config file exists, `BuildConfig::default_config()` is used. Environment variables in paths (`${ANDROID_HOME}`) are expanded by `expand_paths()`. `androidJar` auto-detects the highest API level from `$ANDROID_HOME/platforms/` if not specified.

### Dependency management (`src/dependency.rs`)

Common resource directories shared across configs are detected and precompiled once. Configs are topologically sorted via Kahn's algorithm so dependents build after their dependencies. Independent configs build in parallel via Tokio tasks with a semaphore.

### Resource priority (`src/resource_priority.rs`)

Implements Android's standard resource overlay strategy. When multiple resource directories contain the same resource file, the highest-priority version wins. Priority values: `Library(i)` (0-999), `Additional(i)` (1000-1999), `Main` (2000).

### Caching (`src/cache.rs`)

Two cache types, both SHA-256 based and versioned (`CACHE_VERSION = 1`):
- **`BuildCache`** — per-package cache of source-file→flat-file mappings, stored at `{buildDir}/{packageName}/build-cache.json`. Controls incremental compilation via `needs_recompile()`.
- **`CommonDependencyCache`** — shared cache for resource directories used by multiple configs, stored at `{buildDir}/common-deps/common-dep-cache.json`. Avoids recompiling the same shared `res/` across apps.

### Concurrency

- **Rayon** — parallel `aapt2 compile` per file (pool = CPU cores × 2)
- **Tokio** — parallel multi-config builds (semaphore-limited to `maxParallelBuilds`, default = CPU cores)

## Module Map

| Module | Purpose |
|---|---|
| `src/cli.rs` | Clap CLI definitions, config loading, build dispatch |
| `src/builder.rs` | `SkinBuilder` orchestrating compile → link → finalize |
| `src/aapt2.rs` | `Aapt2` wrapper for `aapt2 compile` and `aapt2 link` subprocesses |
| `src/aar.rs` | Extracts `res/` from AAR files (ZIP archives) |
| `src/cache.rs` | `BuildCache` + `CommonDependencyCache` for incremental builds |
| `src/types.rs` | `BuildConfig`, `AppConfig`, `FlavorConfig`, `MultiAppConfig`, result types |
| `src/dependency.rs` | Common dependency detection and topological ordering |
| `src/resource_priority.rs` | Resource conflict resolution and overlay priority |
| `src/error.rs` | `thiserror` library error types |
| `src/merge.rs` | Library API for merging/extracting `.skin` packages (not used by CLI) |
| `src/lib.rs` | Library entry (re-exports modules; used by tests) |
| `src/main.rs` | Binary entry (tracing init, runs CLI) |

## Tests

- **Unit tests** — in `src/` via `#[cfg(test)] mod tests` and in `tests/unit_tests.rs`
- **Integration tests** — in `tests/integration_test.rs` (config loading and flavor expansion)
- **Example projects** — in `examples/` (not compiled as tests)

Tests that construct `BuildConfig` structs directly (rather than deserializing from JSON) must include all fields — when adding a new field to `BuildConfig`, search for direct struct construction across `src/builder.rs`, `src/cli.rs`, `src/dependency.rs`, and `tests/unit_tests.rs`.

## Key Conventions

- **Error handling**: `anyhow::Result` + `.context()` for application code; `thiserror` in `src/error.rs` for library error types
- **Logging**: `tracing` crate (`info!`, `debug!`, `warn!`, `error!`) — never use `println!` for operational output
- **Terminal output**: `colored` for styled text + `indicatif` progress bars (`.set_message()` between phases)
- **aapt2 calls**: all subprocess calls go through `src/aapt2.rs`; never `Command::new("aapt2")` elsewhere
- **zip 2.x API**: `ZipWriter::start_file::<_, ()>(name, FileOptions::default().compression_method(method))`; `std::io::Write::write_all(&mut writer, &data)`
- **New config fields**: adding a field to `BuildConfig`/`AppConfig`/`FlavorConfig`/`MultiAppConfig` requires updating: `expand_paths()`, `into_build_configs()` inheritance chain, `default_config()`, CLI override loop, and all direct struct constructions in tests (`builder.rs`, `cli.rs`, `dependency.rs`, `tests/unit_tests.rs`)
- **Cache priority**: `cacheDir` (deprecated) > `buildDir` > default `{outputDir}/.build`
