# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add auto changelog generation after release using git-cliff

## [2.0.2] - 2026-02-25

### Documentation

- Add .github/copilot-instructions.md for coding agent onboarding (#21)

### Miscellaneous Tasks

- Update version to 2.0.2 and change edition to 2024 in Cargo.toml

### Performance

- Faster builds via stored ZIP, parallel AAR extraction, hash caching, and O(1) file filtering (#19)

## [2.0.1] - 2026-02-09

### Added

- Make android_jar optional and implement auto-detection from ANDROID_HOME

## [2.0.0] - 2026-02-09

### Added

- High-performance Android skin builder with parallel builds, stable IDs, application-scoped packaging with smart resource filtering, and zero-config mode (#1)
- Include all resources from AAR and additional directories in build output (#3)
- Separate `build_dir` and `output_dir`
- Add quiet mode option to CLI for reduced logging output
- Update documentation and examples for multi-app and multi-module support

### Changed

- Simplify save_failure_log function by removing output_dir parameter and using current working directory

### Documentation

- Clean up README: Remove non-existent features, duplicates, and TypeScript comparison (#2)
- Update README.md

### Fixed

- Add package ID support to fix invalid resource IDs in dynamic loading (#5)

### Styling

- Format code for better readability and consistency

[unreleased]: https://github.com/kagawagao/asb/compare/v2.0.2..HEAD
[2.0.2]: https://github.com/kagawagao/asb/compare/v2.0.1..v2.0.2
[2.0.1]: https://github.com/kagawagao/asb/compare/v2.0.0..v2.0.1
[2.0.0]: https://github.com/kagawagao/asb/releases/tag/v2.0.0

