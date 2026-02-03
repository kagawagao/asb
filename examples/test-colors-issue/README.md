# Test Case: Resources.getIdentifier() Support

This test case demonstrates the fix for using `Resources.getIdentifier()` to lookup resource IDs in skin packages with `additionalResourceDirs`.

## Problem

When using `Resources.getIdentifier()` to find resource IDs in skin packages, it was returning 0 for resources from `additionalResourceDirs`.

## Root Cause

The issue was that values resources need to be properly compiled into `resources.arsc` for `Resources.getIdentifier()` to work. Raw XML files are not needed for this Android API.

## Solution

Values resources are now:
1. ✅ Compiled into `resources.arsc` from ALL directories (main + AAR + additionalResourceDirs)
2. ✅ NOT included as raw XML files in the skin package
3. ✅ Accessible via `Resources.getIdentifier()` with correct resource IDs

This follows Android's standard packaging principles where values resources only exist in compiled form.

## Test Structure

```
test-colors-issue/
├── base/
│   ├── res/
│   │   ├── values/
│   │   │   └── colors.xml          # Base colors (red, green)
│   │   └── values-en/
│   │       └── strings.xml         # English strings
│   └── AndroidManifest.xml
├── feature/
│   ├── res/
│   │   ├── values/
│   │   │   └── colors.xml          # Feature colors (blue)
│   │   └── values-zh/
│   │       └── strings.xml         # Chinese strings
│   └── AndroidManifest.xml
└── asb.config.json
```

## Expected Result

When building `feature.skin`:
- `resources.arsc` contains ALL resources with correct resource IDs
- NO raw values XML files are included
- Only `AndroidManifest.xml` and `resources.arsc` in the skin package
- `Resources.getIdentifier()` works correctly to find all resource IDs

## How Resources.getIdentifier() Works

```java
// Load the skin package
AssetManager assetManager = new AssetManager();
assetManager.addAssetPath(skinPackagePath);
Resources skinResources = new Resources(assetManager, null, null);

// Find resource IDs - these will return correct IDs, not 0
int baseColorId = skinResources.getIdentifier("base_primary", "color", "com.example.skin.feature");
// Returns: 0x7f010000

int featureColorId = skinResources.getIdentifier("feature_primary", "color", "com.example.skin.feature");
// Returns: 0x7f010002

int baseStringId = skinResources.getIdentifier("base_greeting", "string", "com.example.skin.feature");
// Returns: 0x7f020000
```

## Verification

```bash
# Build the skin packages
asb build --config asb.config.json

# Check contents
unzip -l build/feature.skin

# Verify all resources are in resources.arsc
unzip -p build/feature.skin resources.arsc | strings | grep -E "(base_|feature_|Hello|你好)"
```

Expected output:
```
Archive:  build/feature.skin
  Length      Date    Time    Name
---------  ---------- -----   ----
      804  YYYY-MM-DD HH:MM   AndroidManifest.xml
      972  YYYY-MM-DD HH:MM   resources.arsc
---------                     -------
     1776                     2 files

base_primary
base_secondary
feature_primary
Hello
你好
base_greeting
feature_greeting
```
     1776                     2 files

base_primary
base_secondary
feature_primary
Hello
你好
base_greeting
feature_greeting
```
