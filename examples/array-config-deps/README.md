# Multi-App Configuration with Dependencies Example

This example demonstrates using the object-based multi-app configuration with dependencies between configurations. Feature1 and Feature2 both depend on Base resources.

## Configuration

The `asb.config.json` file uses an object format with common fields at the top level and an `apps` array for app-specific configurations. Dependencies are declared using `additionalResourceDirs`:

```json
{
  "outputDir": "./build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0",
  "apps": [
    {
      "resourceDir": "./base/res",
      "manifestPath": "./base/AndroidManifest.xml",
      "packageName": "com.example.skin.base"
    },
    {
      "resourceDir": "./feature1/res",
      "manifestPath": "./feature1/AndroidManifest.xml",
      "packageName": "com.example.skin.feature1",
      "additionalResourceDirs": ["./base/res"]
    },
    {
      "resourceDir": "./feature2/res",
      "manifestPath": "./feature2/AndroidManifest.xml",
      "packageName": "com.example.skin.feature2",
      "additionalResourceDirs": ["./base/res"]
    }
  ]
}
```

## Build Order

ASB will automatically detect the dependency and build in the correct order:

1. Base package (built first since others depend on it)
2. Feature1 and Feature2 packages (built sequentially after base)

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
