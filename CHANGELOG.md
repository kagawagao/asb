# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-01-31

### Changed

- **BREAKING**: Complete rewrite in Rust for ultimate performance
- Replaced TypeScript/Node.js implementation with native Rust binary
- Significantly faster build times and lower memory usage

### Added

- **Concurrent/parallel resource compilation** using Rayon
  - Automatically utilizes all CPU cores
  - Configurable worker thread count
- **Stable resource IDs** support via aapt2's `--stable-ids` parameter
  - Ensures consistent resource IDs across builds
  - Critical for hot update scenarios
- **Multi-app configuration support**
  - Build multiple apps from a single configuration file
  - Parallel build support with configurable concurrency
  - Shared configuration with per-app overrides via `apps` array
- **Enhanced performance**
  - Native binary with minimal overhead
  - Parallel resource processing
  - Optimized file I/O
- **Better error messages** and logging with colored output

### Improved

- Cross-platform binary detection now more reliable
- Incremental build cache more efficient
- Resource file discovery optimized with walkdir

### Technical Details

- Async runtime: Tokio
- Parallel processing: Rayon
- File hashing: SHA2 (SHA-256)
- CLI: Clap v4
- Zero-cost abstractions and compile-time optimizations

## [1.0.0] - 2026-01-31

### Added

- Initial release of Android Skin Builder (asb)
- Cross-platform aapt2 wrapper for resource compilation and linking
- Support for building resource-only skin packages
- AAR dependency extraction and resource merging
- Incremental build support with intelligent caching
- CLI interface with multiple commands:
  - `asb build` - Build skin packages
  - `asb clean` - Clean build artifacts
  - `asb version` - Show aapt2 version
  - `asb init` - Initialize project configuration
- Configuration file support (JSON)
- Comprehensive documentation and examples
- TypeScript implementation with full type definitions
- Support for hot update and plugin scenarios

### Features

- 🎨 Resource-only packaging for hot updates
- 📦 Automatic AAR resource extraction
- 🚀 Incremental builds for faster compilation
- 🔧 Scriptable CLI tool
- 🌐 Cross-platform support (Windows, macOS, Linux)
- ⚡ Direct aapt2 integration

[2.0.0]: https://github.com/kagawagao/asb/releases/tag/v2.0.0
[1.0.0]: https://github.com/kagawagao/asb/releases/tag/v1.0.0
