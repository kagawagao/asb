# Multi-App Configuration with Dependencies Example

This example demonstrates using the object-based multi-app configuration with dependencies between configurations. Feature1 and Feature2 both depend on Base resources. It also showcases the new `baseDir` and `outputFile` features.

## Configuration

The `asb.config.json` file uses an object format with common fields at the top level and an `apps` array for app-specific configurations. Dependencies are declared using `additionalResourceDirs`:

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
      "baseDir": "./base",
      "packageName": "com.example.skin.base",
      "outputFile": "base.skin"
    },
    {
      "baseDir": "./feature1",
      "packageName": "com.example.skin.feature1",
      "additionalResourceDirs": ["./base/res"],
      "outputFile": "feature1.skin"
    },
    {
      "baseDir": "./feature2",
      "packageName": "com.example.skin.feature2",
      "additionalResourceDirs": ["./base/res"],
      "outputFile": "feature2.skin"
    }
  ]
}
```

## New Features Demonstrated

### baseDir Configuration

- Each app specifies only `baseDir` instead of explicit `resourceDir` and `manifestPath`
- `resourceDir` automatically becomes `$baseDir/res`
- `manifestPath` automatically becomes `$baseDir/AndroidManifest.xml`

### outputFile Configuration

- Custom output file names specified per app: `base.skin`, `feature1.skin`, `feature2.skin`
- Without this, outputs would default to `{packageName}.skin`
- Combined with `outputDir` to determine full output path

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

The build will generate three skin packages in the `build` directory with custom names:

- `base.skin`
- `feature1.skin`
- `feature2.skin`

## Project Structure

```
array-config-deps/
├── asb.config.json
├── base/
│   ├── AndroidManifest.xml
│   └── res/
│       └── values/
│           ├── colors.xml    # Base theme colors
│           └── strings.xml   # Base strings
├── feature1/
│   ├── AndroidManifest.xml
│   └── res/
│       └── values/
│           ├── colors.xml    # Feature1-specific colors
│           └── strings.xml   # Feature1 strings
└── feature2/
    ├── AndroidManifest.xml
    └── res/
        └── values/
            ├── colors.xml    # Feature2-specific colors
            └── strings.xml   # Feature2 strings
```

This demonstrates how feature modules can depend on and include resources from a base module, with each module having its own color palette and strings.
