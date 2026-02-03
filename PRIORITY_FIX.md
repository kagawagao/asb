# Resource Priority Fix - Technical Details

## Problem

The initial implementation (v2.1.0) had the Android resource priority order backwards:

**Initial (INCORRECT) Priority:**
1. Main resources (`resourceDir`) - lowest
2. AAR dependencies (`aarFiles`) - medium  
3. Additional resources (`additionalResourceDirs`) - highest

This was **opposite** of Android's standard build system!

## Android Standard

According to [Android Gradle documentation](https://developer.android.com/build/build-variants), the correct priority is:

**Correct Android Priority (LOWEST to HIGHEST):**
1. **Library Dependencies** (AAR files from dependencies)
2. **Main Source Set** (`src/main/res`)
3. **Product Flavor** (e.g., `src/free/res`, `src/pro/res`)
4. **Build Type** (e.g., `src/debug/res`, `src/release/res`)
5. **Build Variant** (e.g., `src/freeDebug/res`) - if exists

In Android, **higher priority resources override lower priority ones**.

## Solution

Fixed the priority order in ASB to match Android standard:

### Code Changes

**1. ResourcePriority Enum** (`src/resource_priority.rs`)

```rust
// BEFORE (WRONG)
pub enum ResourcePriority {
    Main,           // Priority 0 (lowest)
    Aar(usize),     // Priority 1000+
    Additional(usize), // Priority 2000+ (highest)
}

// AFTER (CORRECT)
pub enum ResourcePriority {
    Library(usize),    // Priority 0-999 (lowest) - AAR dependencies
    Main,              // Priority 1000 - main source set
    Additional(usize), // Priority 2000+ (highest) - flavors/build types
}
```

**2. Builder Assignment** (`src/builder.rs`)

```rust
// AAR resources get LOWEST priority
ResourcePriority::Library(idx)

// Main resources get MEDIUM priority  
ResourcePriority::Main

// Additional resources get HIGHEST priority
ResourcePriority::Additional(idx)
```

**3. Base/Overlay Logic**

Smart detection of what should be base vs overlay:

```rust
if has_library {
    // Library resources = base
    // Main + Additional = overlays
} else {
    // Main resources = base
    // Additional = overlays
}
```

This ensures aapt2 always has a base resource set.

## Mapping to Android Gradle

| Android Gradle | ASB Config | Priority | Role |
|---------------|------------|----------|------|
| Library dependencies | `aarFiles` | 0-999 | Lowest - Base (if present) |
| Main source set | `resourceDir` | 1000 | Medium - Base or Overlay |
| Product Flavor | `additionalResourceDirs[0]` | 2000 | High - Overlay |
| Build Type | `additionalResourceDirs[1]` | 2001 | Highest - Overlay |

## Examples

### Example 1: No AAR (Simple Case)

```json
{
  "resourceDir": "./src/main/res",
  "additionalResourceDirs": [
    "./src/free/res",
    "./src/debug/res"
  ]
}
```

**Priority:**
1. `src/main/res` - Base (Main)
2. `src/free/res` - Overlay (Product Flavor)
3. `src/debug/res` - Overlay (Build Type, wins conflicts)

### Example 2: With AAR

```json
{
  "resourceDir": "./src/main/res",
  "aarFiles": ["./libs/theme.aar"],
  "additionalResourceDirs": [
    "./src/free/res",
    "./src/debug/res"
  ]
}
```

**Priority:**
1. `./libs/theme.aar` - Base (Library, lowest)
2. `src/main/res` - Overlay (Main)
3. `src/free/res` - Overlay (Product Flavor)
4. `src/debug/res` - Overlay (Build Type, wins conflicts)

## Verification

All changes verified:
- ✅ Unit tests updated and passing
- ✅ Integration tests passing
- ✅ Example builds work correctly
- ✅ Documentation updated
- ✅ Backward compatible

## References

- [Configure build variants | Android Developers](https://developer.android.com/build/build-variants)
- [Which takes precedence, build types or flavors? | Stack Overflow](https://stackoverflow.com/questions/28745132/which-takes-precedence-gradle-build-types-or-flavors)
- [AAPT2 Documentation](https://developer.android.com/tools/aapt2)

## Impact

This fix ensures ASB follows Android's standard resource merging behavior, making it compatible with how Android developers expect resources to be prioritized in Gradle projects.
