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

## Building Examples

First, build the Rust binary from the root directory:

```bash
# From the root directory
cargo build --release
```

Then each example can be built independently:

```bash
# Navigate to the example directory
cd simple-skin

# Build the skin package (assuming asb is in PATH)
asb build --config asb.config.json

# Or use the binary directly
../../target/release/asb build --config asb.config.json
```

## Creating Your Own Skin

1. Use `asb init` to create a new configuration
2. Organize your resources in a `res/` directory
3. Create an `AndroidManifest.xml`
4. Build with `asb build --config asb.config.json`

## Notes

- All examples assume `ANDROID_HOME` is set in your environment
- The examples use Android API 30, but you can use any API level
- The output APKs are resource-only packages suitable for hot updates

## Need Help?

See the main [README](../README.md) for detailed documentation and usage instructions.
