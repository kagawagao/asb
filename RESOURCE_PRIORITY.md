# Android Resource Priority Strategy - Implementation Documentation

## 概述 / Overview

ASB 从版本 2.1.0 开始实现了完整的 Android 资源优先级策略，用于处理多个资源目录中的资源冲突场景。当多个目录包含同名资源时，系统会按照 Android 标准的优先级规则进行覆盖。

Starting from version 2.1.0, ASB implements complete Android resource priority strategy to handle resource conflicts from multiple directories. When multiple directories contain resources with the same name, the system applies Android's standard priority rules for override.

## 资源优先级规则 / Priority Rules

ASB 按照以下优先级处理资源（数字越大优先级越高）：

1. **主资源目录** (`resourceDir`) - 优先级：0
   - 应用的主要资源目录
   - 最低优先级，会被其他资源覆盖

2. **AAR 依赖资源** (`aarFiles`) - 优先级：1000+
   - AAR 库中的资源
   - 按配置文件中指定的顺序，后面的 AAR 覆盖前面的
   - 优先级：1000, 1001, 1002...

3. **额外资源目录** (`additionalResourceDirs`) - 优先级：2000+
   - 额外指定的资源目录
   - 最高优先级，按顺序覆盖，后面的目录覆盖前面的
   - 优先级：2000, 2001, 2002...

## 技术实现 / Technical Implementation

### aapt2 Overlay Semantics

ASB 使用 aapt2 的原生覆盖语义实现资源优先级：

- **Base Resources**: 主资源目录的资源作为普通参数传递给 `aapt2 link`
- **Overlay Resources**: AAR 和额外资源目录的资源使用 `-R` 标志传递
- **Override Rule**: 根据 aapt2 文档，使用 `-R` 标志的资源具有覆盖语义，最后指定的冲突资源优先

```bash
# Simplified aapt2 link command structure:
aapt2 link \
  base_file1.flat base_file2.flat \      # Base resources
  -R aar_file1.flat -R aar_file2.flat \  # AAR overlay
  -R additional1.flat -R additional2.flat # Additional overlay (highest priority)
```

### Code Architecture

**Key Components:**

1. **ResourcePriority Enum** (`src/resource_priority.rs`):
   ```rust
   pub enum ResourcePriority {
       Main,                    // Priority 0
       Aar(usize),             // Priority 1000+
       Additional(usize),      // Priority 2000+
   }
   ```

2. **link_with_overlays** (`src/aapt2.rs`):
   - New linking function that accepts base and overlay resources separately
   - Applies `-R` flag to overlay resources
   - Maintains proper ordering for priority rules

3. **Builder Updates** (`src/builder.rs`):
   - Tracks flat files by priority during compilation
   - Separates base from overlay resources
   - Passes resources to aapt2 in correct order

## 使用示例 / Usage Examples

### Example 1: 基础资源 + 额外资源覆盖

**Directory Structure:**
```
project/
├── base/res/
│   └── values/
│       └── colors.xml    # primary_color = #FF0000
├── custom/res/
│   └── values/
│       └── colors.xml    # primary_color = #0000FF
└── asb.config.json
```

**Configuration:**
```json
{
  "resourceDir": "./base/res",
  "additionalResourceDirs": ["./custom/res"],
  "manifestPath": "./base/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.app",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar"
}
```

**Result:**
- Final `primary_color` = `#0000FF` (from custom/res, higher priority)
- Custom resources override base resources

### Example 2: 多层覆盖 (Base + AAR + Additional)

**Configuration:**
```json
{
  "resourceDir": "./app/res",
  "aarFiles": [
    "./libs/theme-lib.aar",
    "./libs/ui-lib.aar"
  ],
  "additionalResourceDirs": [
    "./themes/dark/res",
    "./branding/custom/res"
  ]
}
```

**Priority Order (Lowest to Highest):**
1. `./app/res` (Base)
2. `./libs/theme-lib.aar` (AAR #0)
3. `./libs/ui-lib.aar` (AAR #1)
4. `./themes/dark/res` (Additional #0)
5. `./branding/custom/res` (Additional #1 - Highest)

If all sources define `button_color`:
- Final value comes from `./branding/custom/res`

### Example 3: 主题切换场景

**Use Case:** 应用支持多个主题（浅色、深色、高对比度）

**Configuration:**
```json
{
  "resourceDir": "./res",
  "additionalResourceDirs": [
    "./themes/base/res",
    "./themes/dark/res"
  ],
  "packageName": "com.example.app.theme.dark"
}
```

**Resource Override Chain:**
1. `./res` - 应用基础资源
2. `./themes/base/res` - 主题基础样式
3. `./themes/dark/res` - 深色主题覆盖

Build different skin packages by changing `additionalResourceDirs`:
- Light theme: `["./themes/base/res", "./themes/light/res"]`
- Dark theme: `["./themes/base/res", "./themes/dark/res"]`
- High contrast: `["./themes/base/res", "./themes/high-contrast/res"]`

## Build Output / 构建输出

When building with resource priority, ASB provides detailed logs:

```
INFO  Compiling resources from 3 directories...
INFO  Resource compilation complete: 1 base files, 2 overlay sets
INFO  Linking resources with Android resource priority strategy...
INFO  Build completed successfully!
```

The logs show:
- Number of base resource files
- Number of overlay resource sets
- Confirmation of priority strategy application

## 注意事项 / Important Notes

### 1. 资源命名冲突

- **Values Resources** (colors, strings, styles): 完全支持覆盖，同名资源使用高优先级的值
- **File Resources** (layouts, drawables): 同名文件使用高优先级的文件
- **Qualifiers**: 带限定符的资源（如 `-hdpi`, `-v21`）按完整路径匹配

### 2. 部分覆盖

只需在高优先级目录中放置需要覆盖的资源，无需复制所有资源：

```
base/res/values/colors.xml:
  - primary_color
  - secondary_color
  - accent_color

custom/res/values/colors.xml:
  - primary_color  (only override this one)

Result: primary_color from custom, others from base
```

### 3. 新增资源

高优先级目录可以添加基础资源中没有的新资源：

```
base/res/values/colors.xml:
  - primary_color

custom/res/values/colors.xml:
  - custom_highlight_color  (new resource)

Result: Both resources available in final package
```

### 4. AAR 资源优先级

AAR 资源的优先级在主资源和额外资源之间：
- AAR 可以覆盖主资源目录的资源
- 额外资源目录可以覆盖 AAR 资源
- 多个 AAR 按指定顺序处理

## 测试验证 / Testing & Verification

### Test Example

See `examples/resource-priority-test/` for a complete working example demonstrating:
- Three resource directories with overlapping resources
- Proper priority-based override behavior
- Documentation of expected outcomes

### Verify Resource Values

After building, verify the final resource values:

```bash
# List all resources
aapt2 dump resources output.skin | grep color/

# Expected output shows all resources with their IDs
resource 0x7f010000 color/primary_color
resource 0x7f010001 color/secondary_color
...
```

## 性能影响 / Performance Impact

Resource priority implementation has minimal performance impact:

- ✅ **Compilation**: Same performance (separate directory compilation unchanged)
- ✅ **Linking**: Minimal overhead from `-R` flag usage
- ✅ **Runtime**: No impact (standard Android resource loading)

## 向后兼容性 / Backward Compatibility

- ✅ Fully backward compatible with existing configurations
- ✅ If only `resourceDir` is specified (no overlays), behavior unchanged
- ✅ Existing projects work without modification
- ✅ New priority features opt-in through `additionalResourceDirs`

## 故障排除 / Troubleshooting

### Issue: Resources not overriding as expected

**Solution:**
1. Check resource names match exactly (case-sensitive)
2. Verify directory order in `additionalResourceDirs`
3. Check build logs for priority information
4. Ensure qualifiers match (e.g., `values-zh` vs `values`)

### Issue: Build fails with resource conflicts

**Solution:**
- Should not happen with ASB 2.1.0+
- If it does, ensure you're using the latest version
- Check that aapt2 supports `-R` flag (build-tools 28.0.0+)

## 相关文档 / Related Documentation

- [Android Resource Qualifiers](https://developer.android.com/guide/topics/resources/providing-resources)
- [AAPT2 Documentation](https://developer.android.com/tools/aapt2)
- [Runtime Resource Overlays (RRO)](https://source.android.com/docs/core/architecture/rros)

## 更新日志 / Changelog

### Version 2.1.0 (2026-02-03)
- ✨ Implemented Android resource priority strategy
- 🎯 Added support for resource overlay with aapt2's `-R` flag
- 📝 Added ResourcePriority tracking system
- 🔧 New `link_with_overlays` function in aapt2 module
- 📚 Comprehensive documentation and examples
- ✅ All existing tests pass, no breaking changes
