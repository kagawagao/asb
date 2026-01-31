# Multi-Theme Skin Example

This example demonstrates a skin package with day/night theme support.

## Features

- Light theme colors (default)
- Dark theme colors (for night mode)
- Automatic theme switching based on system settings

## Build

```bash
# Install dependencies first
cd ../../
npm install

# Build the example
cd examples/multi-theme
node ../../bin/asb.js build --config asb.config.json
```

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

The built skin package will be in `build/skin-com.example.skin.multitheme.apk`
