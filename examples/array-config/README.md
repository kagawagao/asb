# Array Config Example

This example demonstrates using array mode configuration in ASB, where multiple skin packages are built from a single `asb.config.json` file.

## Configuration

The `asb.config.json` file is an array of build configurations:

```json
[
  {
    "resourceDir": "./app1/res",
    "manifestPath": "./app1/AndroidManifest.xml",
    "outputDir": "./build",
    "packageName": "com.example.skin.app1",
    ...
  },
  {
    "resourceDir": "./app2/res",
    "manifestPath": "./app2/AndroidManifest.xml",
    "outputDir": "./build",
    "packageName": "com.example.skin.app2",
    ...
  }
]
```

## Building

To build all skin packages:

```bash
cd examples/array-config
asb build
```

This will build both `app1` and `app2` skin packages in parallel, since they have no dependencies on each other.

## Output

The build will generate two skin packages in the `build` directory:
- `com.example.skin.app1.skin`
- `com.example.skin.app2.skin`
