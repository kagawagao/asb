# Full Android Multi-Module Project Example

This is a comprehensive example demonstrating how to use ASB with a realistic Android multi-module project structure.

## Project Structure

```
full-android-app/
├── app/                    # Main application module
│   ├── res/
│   │   ├── values/
│   │   │   ├── colors.xml
│   │   │   └── strings.xml
│   │   ├── drawable-hdpi/
│   │   │   └── ic_launcher.png (placeholder)
│   │   └── layout/
│   │       └── activity_main.xml
│   └── AndroidManifest.xml
│
├── feature-home/           # Home feature module
│   ├── res/
│   │   ├── values/
│   │   │   ├── colors.xml
│   │   │   └── strings.xml
│   │   └── layout/
│   │       └── fragment_home.xml
│   └── AndroidManifest.xml
│
├── feature-profile/        # Profile feature module
│   ├── res/
│   │   ├── values/
│   │   │   ├── colors.xml
│   │   │   └── strings.xml
│   │   └── layout/
│   │       └── fragment_profile.xml
│   └── AndroidManifest.xml
│
├── feature-settings/       # Settings feature module
│   ├── res/
│   │   ├── values/
│   │   │   ├── colors.xml
│   │   │   └── strings.xml
│   │   └── layout/
│   │       └── fragment_settings.xml
│   └── AndroidManifest.xml
│
└── asb.multi-module.json  # Multi-module configuration
```

## Building the Project

### 1. Build from Root

First, ensure ASB is built:

```bash
cd /path/to/asb
cargo build --release
```

### 2. Build Individual Modules

You can build each module separately:

```bash
# Build app module
cd examples/full-android-app
../../target/release/asb build \
  --resource-dir app/res \
  --manifest app/AndroidManifest.xml \
  --output build/app \
  --package com.example.skinapp.app \
  --android-jar $ANDROID_HOME/platforms/android-30/android.jar

# Build feature-home module
../../target/release/asb build \
  --resource-dir feature-home/res \
  --manifest feature-home/AndroidManifest.xml \
  --output build/feature-home \
  --package com.example.skinapp.home \
  --android-jar $ANDROID_HOME/platforms/android-30/android.jar
```

### 3. Build All Modules and Merge

Use the multi-module configuration to build and merge all modules:

```bash
cd examples/full-android-app
../../target/release/asb build-multi --config asb.multi-module.json
```

This will:
1. Build each module into separate APKs
2. Merge all modules into a single file: `build/merged-skin.skin`

### 4. Using the Merged Skin Package

The merged skin package can be distributed and extracted by your Android app:

```
ASB_MERGED_V1
4
app|<size>
<binary APK data>
feature-home|<size>
<binary APK data>
feature-profile|<size>
<binary APK data>
feature-settings|<size>
<binary APK data>
```

## Features Demonstrated

- **Multi-module architecture**: Separate modules for different features
- **Resource organization**: Each module has its own resources
- **Color theming**: Module-specific color schemes
- **Layouts**: Module-specific UI layouts
- **Stable IDs**: Consistent resource IDs across builds (add `--stable-ids` flag)
- **Incremental builds**: Fast rebuilds with caching
- **Parallel compilation**: Utilizes multiple CPU cores

## Advanced Usage

### With Stable IDs

Create a `stable-ids.txt` file to maintain consistent resource IDs:

```bash
../../target/release/asb build-multi \
  --config asb.multi-module.json \
  --stable-ids stable-ids.txt
```

### With Custom Worker Count

Control parallel compilation threads:

```bash
../../target/release/asb build-multi \
  --config asb.multi-module.json \
  --workers 16
```

### Incremental Builds

Enable caching for faster subsequent builds:

```json
{
  "modules": [...],
  "incremental": true,
  "mergedOutput": "./build/merged-skin.skin"
}
```

## Use Cases

1. **Dynamic Feature Modules**: Build skin packages for Android Dynamic Feature Modules
2. **Plugin Systems**: Create skinnable plugins for modular apps
3. **Hot Updates**: Update app theming without reinstalling
4. **A/B Testing**: Distribute different themes to different user groups
5. **White Labeling**: Quickly rebrand apps for different clients

## Notes

- Ensure `ANDROID_HOME` environment variable is set
- Each module should have a unique package name
- Resource names should be unique across modules or properly namespaced
- The merged output format is optimized for efficient extraction
