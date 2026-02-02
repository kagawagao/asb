# Full Android Application Example

This is a comprehensive example demonstrating how to use ASB with a realistic Android application that includes multiple feature modules.

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
├── feature-home/           # Home feature module resources
│   └── res/
│       ├── values/
│       │   ├── colors.xml
│       │   └── strings.xml
│       └── layout/
│           └── fragment_home.xml
│
├── feature-profile/        # Profile feature module resources
│   └── res/
│       ├── values/
│       │   ├── colors.xml
│       │   └── strings.xml
│       └── layout/
│           └── fragment_profile.xml
│
└── feature-settings/       # Settings feature module resources
    └── res/
        ├── values/
        │   ├── colors.xml
        │   └── strings.xml
        └── layout/
            └── fragment_settings.xml
```

## Building the Application

ASB builds skin packages at the application level, automatically including resources from all feature modules and their dependencies.

### 1. Build from Root

First, ensure ASB is built:

```bash
cd /path/to/asb
cargo build --release
```

### 2. Build Application Skin Package

Build the entire application with all its resources:

```bash
cd examples/full-android-app
../../target/release/asb build \
  --resource-dir app/res \
  --manifest app/AndroidManifest.xml \
  --output build \
  --package com.example.skinapp.app \
  --android-jar $ANDROID_HOME/platforms/android-34/android.jar
```

This creates `build/com.example.skinapp.app.skin` containing:
- All resources from the app module
- Automatically indexed dependent resources from feature modules
- resources.arsc (compiled resources)
- AndroidManifest.xml
- All XML resource files (layouts, values, etc.)

### 3. Using Configuration File

Create an `asb.config.json` for the app:

```json
{
  "resourceDir": "./app/res",
  "manifestPath": "./app/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skinapp.app",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "additionalResourceDirs": [
    "./feature-home/res",
    "./feature-profile/res",
    "./feature-settings/res"
  ],
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0"
}
```

Then build with:

```bash
../../target/release/asb build --config asb.config.json
```

## Features Demonstrated

- **Application-scoped packaging**: Build complete skin packages per application
- **Automatic dependency resolution**: Resources from all modules are automatically included
- **Resource organization**: Modular resource structure
- **Color theming**: Module-specific color schemes
- **Layouts**: Module-specific UI layouts
- **Stable IDs**: Consistent resource IDs across builds (add `--stable-ids` flag)
- **Incremental builds**: Fast rebuilds with caching
- **Parallel compilation**: Utilizes multiple CPU cores

## Advanced Usage

### With Stable IDs

Create a `stable-ids.txt` file to maintain consistent resource IDs:

```bash
../../target/release/asb build \
  --config asb.config.json \
  --stable-ids stable-ids.txt
```

### With Custom Worker Count

Control parallel compilation threads:

```bash
../../target/release/asb build \
  --config asb.config.json \
  --workers 16
```

### With Third-Party Dependencies

If your application references resources from AAR libraries:

```json
{
  "resourceDir": "./app/res",
  "manifestPath": "./app/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skinapp.app",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "aarFiles": [
    "./libs/support-lib.aar",
    "./libs/material-components.aar"
  ]
}
```

The build process automatically:
- Extracts AAR resources
- Includes referenced resources in the skin package
- Handles resource ID conflicts properly

## Output File Naming

The output skin package is named using the application's package name:

- Package: `com.example.skinapp.app`
- Output: `com.example.skinapp.app.skin`

This makes it easy to identify which application each skin package belongs to.

## Use Cases

1. **Modular Applications**: Build skins for applications with multiple feature modules
2. **Dynamic Theming**: Update app theming without reinstalling
3. **Hot Updates**: Distribute theme updates over-the-air
4. **A/B Testing**: Distribute different themes to different user groups
5. **White Labeling**: Quickly rebrand apps for different clients

## Notes

- Ensure `ANDROID_HOME` environment variable is set
- The application package name determines the output filename
- All referenced resources (from feature modules, AARs, etc.) are automatically included
- Resource names should be unique or properly namespaced
- The skin package format is optimized for efficient loading in Android
