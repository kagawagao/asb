# Multi-App Configuration Example

This example demonstrates using the new object-based multi-app configuration format in ASB, where multiple skin packages are built from a single `asb.config.json` file with common configuration extracted to the top level.

## Configuration

The `asb.config.json` file uses an object format with common fields at the top level and an `apps` array for app-specific configurations. It also demonstrates the new `baseDir` feature which provides automatic defaults:

```json
{
  "baseDir": "./",
  "outputDir": "./build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0",
  "apps": [
    {
      "baseDir": "./app1",
      "packageName": "com.example.skin.app1"
    },
    {
      "baseDir": "./app2",
      "packageName": "com.example.skin.app2"
    }
  ]
}
```

## New Features Demonstrated

### baseDir Configuration
When `baseDir` is specified (either at the top level or per app):
- `resourceDir` defaults to `$baseDir/res`
- `manifestPath` defaults to `$baseDir/AndroidManifest.xml`

This eliminates the need to specify these paths explicitly when following standard Android project structure.

### Benefits

- **No Duplication**: Common configuration (outputDir, androidJar, incremental, etc.) is defined once at the top level
- **Clean Structure**: Each app only needs to specify `baseDir` and `packageName` when following standard structure
- **Object Format**: The config file remains an object (not an array), making it easier to extend with additional top-level settings
- **Flexible**: Can still override `resourceDir` or `manifestPath` per app if needed

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
