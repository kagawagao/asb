# ASB Examples

This directory contains example projects demonstrating different use cases of the Android Skin Builder (asb) tool.

## Examples

### 1. Simple Skin (`simple-skin/`)

A basic example showing how to create a simple skin package with colors and strings.

**Features:**
- Basic color definitions
- String resources
- Minimal configuration

**Use case:** Getting started with asb, creating basic theme packages

### 2. Multi-Theme Skin (`multi-theme/`)

An advanced example demonstrating day/night theme support with multiple resource qualifiers.

**Features:**
- Light theme colors
- Dark theme colors (values-night)
- Automatic theme switching

**Use case:** Creating adaptive themes that respond to system settings

### 3. Full Android Multi-Module App (`full-android-app/`)

A comprehensive, realistic Android project with multiple feature modules demonstrating enterprise-level skin packaging.

**Modules:**
- **app**: Main application module with launcher activity and navigation
- **feature-home**: Home screen with cards, recent items, and quick actions
- **feature-profile**: User profile with avatar, statistics (posts, followers, following)
- **feature-settings**: Complete settings UI with account, notifications, appearance, and about sections

**Features:**
- Multi-module architecture (4 independent modules)
- Real-world Android layouts and components
- Module-specific theming and resources
- Build-multi support with automatic merging
- Parallel compilation across modules
- Incremental builds with caching
- Stable resource IDs for hot updates

**Use case:** 
- Enterprise Android apps with dynamic feature modules
- Plugin systems with independent feature packages
- Hot update systems for modular apps
- White-labeling and rebranding at scale

## Building Examples

First, build the Rust binary from the root directory:

```bash
# From the root directory
cargo build --release
```

### Building Single-Module Examples

Each example can be built independently:

```bash
# Navigate to the example directory
cd simple-skin

# Build the skin package (assuming asb is in PATH)
asb build --config asb.config.json

# Or use the binary directly
../../target/release/asb build --config asb.config.json
```

### Building Multi-Module Example

For the full-android-app example, use build-multi:

```bash
cd full-android-app

# Build all modules and merge them
../../target/release/asb build-multi --config asb.multi-module.json

# This creates:
# - build/app/app.skin
# - build/feature-home/feature-home.skin
# - build/feature-profile/feature-profile.skin
# - build/feature-settings/feature-settings.skin
# - build/merged-skin.asb (all modules merged)
```

## Creating Your Own Skin

1. Use `asb init` to create a new configuration
2. Organize your resources in a `res/` directory
3. Create an `AndroidManifest.xml`
4. Build with `asb build --config asb.config.json`

For multi-module projects:
1. Create separate directories for each module
2. Create `asb.multi-module.json` with module configurations
3. Build with `asb build-multi --config asb.multi-module.json`

## Notes

- All examples assume `ANDROID_HOME` is set in your environment
- The examples use Android API 30, but you can use any API level
- The output APKs are resource-only packages suitable for hot updates
- Multi-module example demonstrates realistic Android app structure

## Need Help?

See the main [README](../README.md) for detailed documentation and usage instructions.
