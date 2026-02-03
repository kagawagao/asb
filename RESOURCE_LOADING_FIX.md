# Complete Resource Loading Fix - Technical Documentation

This document explains the complete fix for resource loading issues in ASB-generated skin packages.

## Overview of Issues and Solutions

### Issue 1: Resources.getIdentifier() Returns 0
**Problem:** When loading skin packages dynamically, `Resources.getIdentifier()` returned 0 instead of valid resource IDs.

**Root Cause:** When a package name was changed (via `--package` or config), only the manifest package was renamed using `--rename-manifest-package`. The resource package (used in R.java and resource IDs) was not renamed, causing a mismatch.

**Solution:** Added `--rename-resources-package` flag alongside `--rename-manifest-package` to ensure both manifest and resource packages are renamed consistently.

**File Changed:** `src/aapt2.rs` (line 290)
```rust
if let Some(pkg) = package_name {
    cmd.arg("--rename-manifest-package").arg(pkg);
    cmd.arg("--rename-resources-package").arg(pkg);  // Added
}
```

### Issue 2: FileNotFoundException When Accessing Resources
**Problem:** After fixing Issue 1, `getIdentifier()` worked but accessing actual resource files threw FileNotFoundException.

**Root Cause:** The `add_resources_to_apk` function was:
1. Removing aapt2's compiled binary XML resources from the APK
2. Replacing them with raw text XML files from source directories

Android expects compiled binary XML format, not text XML, so it couldn't load the resources.

**Solution:** Removed the post-processing code and trusted aapt2's output. aapt2 already compiles all resources to the correct binary format.

**File Changed:** `src/builder.rs` - Simplified `add_resources_to_apk` to be a no-op

## Technical Details

### Android Resource Loading Mechanism

When Android loads resources at runtime:

1. **Resource ID Lookup:** `Resources.getIdentifier(name, type, packageName)`
   - Searches in resources.arsc for the resource by name and type
   - Uses the package name to identify the correct package group
   - Returns the resource ID (format: 0xPPTTEEEE)

2. **Resource File Access:** Once you have the ID, Android loads the actual file:
   - For values resources (colors, strings): Data is in resources.arsc
   - For layouts, drawables: Android loads the file from res/ in the APK
   - Files MUST be in compiled binary XML format

### Binary XML Format

Android uses a custom binary XML format for efficiency:

**Text XML (Source):**
```xml
<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="...">
    <TextView android:text="@string/hello" />
</LinearLayout>
```

**Binary XML (Compiled by aapt2):**
- Starts with magic bytes: `0x03 0x00 0x08 0x00`
- String pool for efficient string storage
- Resource references as integers, not text
- Compressed and optimized for fast parsing

**Verification:**
```bash
# Check if XML is binary (should show hex, not text)
unzip -p skin.apk res/layout/main.xml | xxd | head

# Parse binary XML to see structure
aapt dump xmltree skin.apk res/layout/main.xml
```

### Package Renaming

Android APKs have two related package identifiers:

1. **Manifest Package:** Declared in AndroidManifest.xml
   - Controlled by `--rename-manifest-package`
   - Visible to PackageManager
   - Used for app installation and identification

2. **Resource Package:** Used in resources.arsc and R.java
   - Controlled by `--rename-resources-package`
   - Used for resource lookups with `getIdentifier()`
   - Part of every resource ID

**Both must match for dynamic resource loading to work!**

## Complete Working Example

### 1. Build Skin Package with Renamed Package

**Config (asb.config.json):**
```json
{
  "resourceDir": "./res",
  "manifestPath": "./AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.newskin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "packageId": "0x7f"
}
```

**Build:**
```bash
asb build --config asb.config.json
```

### 2. Android Code to Load Skin

```java
public class SkinLoader {
    private Resources skinResources;
    private String skinPackageName = "com.example.newskin";
    
    public boolean loadSkin(Context context, String skinPath) {
        try {
            // 1. Create AssetManager and add skin package
            AssetManager assetManager = AssetManager.class.newInstance();
            Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
            int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
            
            if (cookie == 0) {
                Log.e("SkinLoader", "Failed to load skin: " + skinPath);
                return false;
            }
            
            // 2. Create Resources object
            skinResources = new Resources(
                assetManager,
                context.getResources().getDisplayMetrics(),
                context.getResources().getConfiguration()
            );
            
            // 3. Get resource ID (will work because packages are renamed)
            int colorId = skinResources.getIdentifier(
                "primary_color",        // Resource name
                "color",                // Resource type
                skinPackageName         // MUST match skin package name
            );
            
            if (colorId == 0) {
                Log.e("SkinLoader", "Resource not found");
                return false;
            }
            
            Log.d("SkinLoader", "Resource ID: 0x" + Integer.toHexString(colorId));
            // Output: Resource ID: 0x7f010004
            
            // 4. Access the resource (will work because binary XML is present)
            int color = skinResources.getColor(colorId, null);
            Log.d("SkinLoader", "Color value: 0x" + Integer.toHexString(color));
            
            // 5. Load layout resources (binary XML will be parsed)
            int layoutId = skinResources.getIdentifier(
                "activity_main",
                "layout",
                skinPackageName
            );
            
            if (layoutId != 0) {
                // This works because layout XML is in binary format
                XmlResourceParser parser = skinResources.getLayout(layoutId);
                Log.d("SkinLoader", "Layout loaded successfully");
            }
            
            return true;
            
        } catch (Exception e) {
            Log.e("SkinLoader", "Error loading skin", e);
            return false;
        }
    }
}
```

### 3. Verification

```bash
# Verify manifest package
aapt dump badging build/com.example.newskin.skin | grep "^package:"
# Output: package: name='com.example.newskin' ...

# Verify resource package  
aapt dump resources build/com.example.newskin.skin | grep "Package Group"
# Output: Package Group 0 id=0x7f name=com.example.newskin

# Verify layout is binary XML
aapt dump xmltree build/com.example.newskin.skin res/layout/activity_main.xml
# Output: Shows parsed XML structure (proves it's binary XML)

# Verify all resources use correct package
aapt dump resources build/com.example.newskin.skin | grep "spec resource" | head -3
# Output: 
#   spec resource 0x7f010000 com.example.newskin:color/primary_color
#   spec resource 0x7f010001 com.example.newskin:color/accent_color
#   ...
```

## Troubleshooting

### getIdentifier() Still Returns 0

**Check:**
1. Is `packageId` set to `"0x7f"` in config?
2. Does the package name in code match the skin package exactly?
3. Are you building with ASB that includes both fixes?

```bash
# Verify resource package name
aapt dump resources your-skin.skin | grep "Package Group"
```

### FileNotFoundException on Resource Access

**Check:**
1. Are resources in binary XML format?
   ```bash
   unzip -p skin.skin res/layout/main.xml | xxd | head -1
   # Should show: 00000000: 0300 0800 ...
   ```

2. Does the APK contain the resource files?
   ```bash
   unzip -l skin.skin | grep "res/layout"
   ```

3. Can aapt parse the resource?
   ```bash
   aapt dump xmltree skin.skin res/layout/main.xml
   # Should show XML structure, not an error
   ```

### Drawable Resources Not Loading

Drawables should also be in the APK and accessible:

```bash
# Check drawable files are present
unzip -l skin.skin | grep "res/drawable"

# For PNG/JPG: Files are kept as-is (not compiled)
# For XML drawables: Compiled to binary XML like layouts
```

## Performance Considerations

### Binary XML Benefits

1. **Faster Parsing:** Binary format is 3-5x faster to parse than text XML
2. **Smaller Size:** String pooling reduces duplicate strings
3. **Efficient Lookups:** Resource references are integers, not string lookups
4. **Memory Efficient:** Optimized memory layout for Android's XML parser

### Package ID Considerations

The Package ID (`0x7f`) is standard for apps. Other values:
- `0x01`: Android system resources
- `0x7e`, `0x7d`: Some plugin frameworks use these
- `0x02-0x7e`: Available for custom use

**Best Practice:** Use `0x7f` unless you have a specific plugin framework requirement.

## Summary

Both fixes are required for proper resource loading:

1. **--rename-resources-package:** Ensures resource IDs can be found
2. **Binary XML format:** Ensures resource files can be loaded

After both fixes:
- ✅ `getIdentifier()` returns valid IDs
- ✅ Resource files load successfully  
- ✅ Layouts, drawables, and other resources work
- ✅ Package renaming works correctly
- ✅ Performance is optimal (binary XML)

## References

- [Android AAPT2 Documentation](https://developer.android.com/tools/aapt2)
- [Android Resources Overview](https://developer.android.com/guide/topics/resources/providing-resources)
- [Binary XML Format](https://justanapplication.wordpress.com/category/android/android-binary-xml/)
