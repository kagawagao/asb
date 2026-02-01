# Simple Skin Example

This example demonstrates a basic skin package with colors and strings.

## Build

```bash
# Build the example
cd examples/simple-skin
asb build --config asb.config.json
```

**Note:** Make sure you have built the Rust binary first with `cargo build --release` from the root directory, and that `asb` is in your PATH or use `../../target/release/asb` instead.

## Structure

```
simple-skin/
├── res/
│   └── values/
│       ├── colors.xml
│       └── strings.xml
├── AndroidManifest.xml
└── asb.config.json
```

## Output

The built skin package will be in `build/skin-com.example.skin.simple.apk`
