# Resource Priority Test Example

This example demonstrates Android resource priority strategy implementation in ASB.

## Structure

```
resource-priority-test/
├── base/                   # Main resource directory (base priority, like src/main/res)
│   └── res/
│       └── values/
│           └── colors.xml  # Defines: primary_color=#FF0000, secondary_color=#00FF00, accent_color=#0000FF
├── overlay/                # First additional directory (Product Flavor priority)
│   └── res/
│       └── values/
│           └── colors.xml  # Overrides: primary_color=#00FF00
└── additional/             # Second additional directory (Build Type - highest priority)
    └── res/
        └── values/
            └── colors.xml  # Overrides: primary_color=#0000FF, adds: new_color=#FFFF00
```

## Expected Behavior

According to Android resource priority rules (符合 Android 标准):
1. **Main resource directory** (`base/res`) - medium priority (like src/main/res)
2. **Additional resource directories** - higher priority (like Product Flavor and Build Type)
3. Later additional directories override earlier ones

**Note:** In this example, there are no AAR/Library dependencies, so `base/res` (Main) is the base resources, and `overlay/` and `additional/` are overlays with increasing priority.

### Expected Final Resource Values

- `primary_color`: `#0000FF` (from `additional/res`, highest priority - like Build Type)
- `secondary_color`: `#00FF00` (from `base/res`, no override)
- `accent_color`: `#0000FF` (from `base/res`, no override)
- `new_color`: `#FFFF00` (from `additional/res`, unique)

## Build Command

```bash
cd examples/resource-priority-test
asb build
```

## Expected Output

ASB should:
1. Use `base/res` as base resources (Main)
2. Apply `overlay/res` and `additional/res` as overlays in order
3. Apply Android priority rules (later directories override earlier ones)
4. Generate a skin package with the correct final resource values

Example log output:
```
INFO  Resource conflicts resolved: 2 overrides detected
INFO    res/values/colors.xml#primary_color overridden by additional/res/values/colors.xml (from Main to Additional(1))
INFO  Resource compilation complete: 4 unique resources (2 conflicts resolved)
```

## Verification

After building, you can verify the resource values using aapt:

```bash
aapt dump resources build/com.example.priority.test.skin | grep color
```

The output should show `primary_color` has the value from `additional/res` (highest priority).

## Testing Different Scenarios

1. **Modify priority order**: Change order in `additionalResourceDirs`
2. **Add more conflicts**: Create overlapping resources in drawable, layout, etc.
3. **Test with AAR**: Add AAR dependencies to test AAR priority vs additional dirs
