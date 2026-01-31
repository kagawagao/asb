# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- TypeScript support with full type definitions
- Support for hot update and plugin scenarios

### Features
- 🎨 Resource-only packaging for hot updates
- 📦 Automatic AAR resource extraction
- 🚀 Incremental builds for faster compilation
- 🔧 Scriptable CLI tool
- 🌐 Cross-platform support (Windows, macOS, Linux)
- ⚡ Direct aapt2 integration

[1.0.0]: https://github.com/kagawagao/asb/releases/tag/v1.0.0
