# Flavors Example

This example demonstrates the flavors feature in ASB, where a single app configuration can generate multiple build variants with different configurations.

## Configuration

The `asb.config.json` file shows how to define flavors:

```json
{
  "outputDir": "./build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0",
  "apps": [
    {
      "baseDir": "./",
      "packageName": "com.example.skin.myapp",
      "flavors": [
        {
          "name": "free",
          "outputFile": "myapp-free.skin"
        },
        {
          "name": "pro",
          "outputFile": "myapp-pro.skin",
          "versionCode": 2
        }
      ]
    }
  ]
}
```

## Flavors Feature

### What are Flavors?

Flavors allow you to create multiple variants of the same app from a single configuration. Each flavor:

- Creates a separate build task
- Can override any app-level configuration
- Builds in parallel with other flavors for optimal performance

### How it Works

1. **Base Configuration**: Define common settings at the app level (baseDir, packageName, etc.)
2. **Flavor Overrides**: Each flavor can override specific settings (packageName, outputFile, versionCode, etc.)
3. **Automatic Package Names**: If a flavor doesn't specify packageName, it defaults to `{app.packageName}.{flavor.name}`

### Flavor Configuration Priority

Values are resolved in this order (highest priority first):

1. Flavor-specific value
2. App-level value
3. Common/top-level value

## This Example

This configuration creates two build variants:

### Free Flavor

- Package: `com.example.skin.myapp.free` (auto-generated)
- Output: `myapp-free.skin`
- Version: 1 (from common config)

### Pro Flavor

- Package: `com.example.skin.myapp.pro` (auto-generated)
- Output: `myapp-pro.skin`
- Version: 2 (overridden)

## Building

To build all flavors:

```bash
cd examples/flavors-example
asb build
```

This will build both free and pro flavors in parallel.

## Output

The build will generate two skin packages in the `build` directory:

- `myapp-free.skin`
- `myapp-pro.skin`

## Project Structure

```
flavors-example/
├── asb.config.json
├── AndroidManifest.xml
└── res/
    └── values/
        ├── colors.xml    # Theme colors
        └── strings.xml   # App strings
```

Both flavors share the same resources, but can be extended to have flavor-specific resource overlays if needed.

## Use Cases

Flavors are useful for:

- **Free vs Paid versions**: Different package names, resources, or configurations
- **Environment variants**: dev, staging, production with different endpoints or settings
- **Regional variants**: Different resources or configurations for different markets
- **Feature variants**: Enabling/disabling features through different configurations
