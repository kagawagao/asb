# Test Case: Colors Issue

This test case demonstrates the fix for values resources in skin packages with `additionalResourceDirs`.

## Problem

When building a skin package with `additionalResourceDirs`, values resources (like colors, strings, etc.) from the additional directories were being compiled into `resources.arsc` but NOT accessible at runtime.

## Root Cause

The skin package was incorrectly including raw XML files from the values folder. When multiple resource directories had the same file path (e.g., `base/res/values/colors.xml` and `feature/res/values/colors.xml`), only one raw XML file would be included, causing conflicts.

## Solution

Values resources are now:
1. ✅ Compiled into `resources.arsc` from ALL directories (main + AAR + additionalResourceDirs)
2. ✅ NOT included as raw XML files in the skin package

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
- `resources.arsc` contains ALL resources (base_primary, base_secondary, feature_primary, Hello, 你好)
- NO raw values XML files are included
- Only `AndroidManifest.xml` and `resources.arsc` in the skin package

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
      804  2026-02-03 02:43   AndroidManifest.xml
      972  2026-02-03 02:43   resources.arsc
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
