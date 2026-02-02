# Array Config with Dependencies Example

This example demonstrates using array mode configuration with dependencies between configurations. Feature1 and Feature2 both depend on Base resources.

## Configuration

The `asb.config.json` file contains three configurations where feature1 and feature2 depend on base:

```json
[
  {
    "resourceDir": "./base/res",
    "packageName": "com.example.skin.base",
    ...
  },
  {
    "resourceDir": "./feature1/res",
    "packageName": "com.example.skin.feature1",
    "additionalResourceDirs": ["./base/res"],
    ...
  },
  {
    "resourceDir": "./feature2/res",
    "packageName": "com.example.skin.feature2",
    "additionalResourceDirs": ["./base/res"],
    ...
  }
]
```

## Build Order

ASB will automatically detect the dependency and build in the correct order:

1. Base package (built first since others depend on it)
2. Feature1 and Feature2 packages (built in parallel or sequentially after base)

## Building

To build all skin packages:

```bash
cd examples/array-config-deps
asb build
```

## Output

The build will generate three skin packages in the `build` directory:
- `com.example.skin.base.skin`
- `com.example.skin.feature1.skin`
- `com.example.skin.feature2.skin`
