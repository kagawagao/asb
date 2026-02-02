# Multi-Theme Skin Example

This example demonstrates a skin package with day/night theme support.

## Features

- Light theme colors (default)
- Dark theme colors (for night mode)
- Automatic theme switching based on system settings

## Build

```bash
# Build the example
cd examples/multi-theme
asb build --config asb.config.json
```

**Note:** Make sure you have built the Rust binary first with `cargo build --release` from the root directory, and that `asb` is in your PATH or use `../../target/release/asb` instead.

## Structure

```
multi-theme/
├── res/
│   ├── values/
│   │   ├── colors.xml       # Light theme
│   │   └── strings.xml
│   └── values-night/
│       └── colors.xml       # Dark theme
├── AndroidManifest.xml
└── asb.config.json
```

## Usage in App

When this skin package is loaded by your app, it will automatically use:
- Light theme colors during the day
- Dark theme colors at night (or when dark mode is enabled)

## Output

The built skin package will be in `build/skin-com_example_skin_multitheme.skin`
