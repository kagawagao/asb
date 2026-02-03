# Package Renaming Example

This example demonstrates how ASB now correctly renames both the manifest package and the resource package when you specify a different package name.

## Background

Android APKs have two related but distinct package identifiers:
1. **Manifest package**: The package declared in `AndroidManifest.xml` (`package="..."`)
2. **Resource package**: The package used in generated R.java and resource identifiers

Previously, ASB only renamed the manifest package, which could cause inconsistencies.

## Testing the Feature

### Setup
Start with the `simple-skin` example which has:
- Original package: `com.example.skin.simple`

### Test 1: Build with Original Package

```bash
cd examples/simple-skin
../../target/release/asb build --config asb.config.json
```

Verify the package names:
```bash
aapt dump badging build/com.example.skin.simple.skin | grep package
# Output: package: name='com.example.skin.simple' ...

aapt dump resources build/com.example.skin.simple.skin | grep "Package Group"
# Output: Package Group 0 id=0x7f packageCount=1 name=com.example.skin.simple
```

### Test 2: Build with Renamed Package

```bash
../../target/release/asb build --config asb.config.json --package com.example.newskin.test
```

Verify both packages are renamed:
```bash
aapt dump badging build/com.example.newskin.test.skin | grep package
# Output: package: name='com.example.newskin.test' ...

aapt dump resources build/com.example.newskin.test.skin | grep "Package Group"
# Output: Package Group 0 id=0x7f packageCount=1 name=com.example.newskin.test
```

### Test 3: Verify Resource Names

Check that individual resources also use the renamed package:
```bash
aapt dump resources build/com.example.newskin.test.skin | grep "spec resource"
```

Output shows all resources use the new package name:
```
spec resource 0x7f010000 com.example.newskin.test:color/backgroundColor
spec resource 0x7f010001 com.example.newskin.test:color/colorAccent
spec resource 0x7f010002 com.example.newskin.test:color/colorPrimary
...
```

## The Fix

The fix was simple but important. In `src/aapt2.rs`, when a package name is provided:

**Before:**
```rust
if let Some(pkg) = package_name {
    cmd.arg("--rename-manifest-package").arg(pkg);
}
```

**After:**
```rust
if let Some(pkg) = package_name {
    cmd.arg("--rename-manifest-package").arg(pkg);
    cmd.arg("--rename-resources-package").arg(pkg);
}
```

This ensures both the manifest AND resource packages are renamed consistently.

## Why This Matters

When loading resources dynamically (e.g., for skin/theme switching), Android's `Resources.getIdentifier()` uses the resource package name:

```java
// This needs the correct resource package name to work
int resId = resources.getIdentifier("colorPrimary", "color", packageName);
```

If the resource package doesn't match what you expect, `getIdentifier()` returns 0, causing resource loading to fail.

## Verification Commands

Use these commands to verify package renaming:

```bash
# Check manifest package
aapt dump badging <skin.apk> | grep "^package:"

# Check resource package
aapt dump resources <skin.apk> | grep "Package Group"

# Check individual resource names
aapt dump resources <skin.apk> | grep "spec resource" | head -5
```

All three should show the same (renamed) package name.
