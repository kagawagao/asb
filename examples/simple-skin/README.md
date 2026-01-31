# Simple Skin Example

This example demonstrates a basic skin package with colors and strings.

## Build

```bash
# Install dependencies first
cd ../../
npm install

# Build the example
cd examples/simple-skin
node ../../bin/asb.js build --config asb.config.json
```

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
