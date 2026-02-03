# Resources.getIdentifier() 返回 0 问题解决方案

## 问题描述

当前输出的产物在运行时加载时，通过 `new Resources()` 创建后，从 `Resources.getIdentifier()` 读取到的 `resourceId` 为 `0`。

## 问题原因分析

### 1. Android 资源 ID 的结构

Android 资源 ID 是一个 32 位整数，格式为：`0xPPTTEEEE`

```
示例: 0x7f010004
      │  │  └─ Entry ID (资源条目编号 0004)
      │  └─── Type ID (资源类型编号 01 = color)
      └────── Package ID (包标识符 7f)
```

### 2. Package ID 的重要性

- **0x01**: Android 系统资源 (android.R.xxx)
- **0x7f**: 标准 Android 应用资源 (R.xxx) ← 这是关键！
- **其他值**: 特殊插件化框架使用

### 3. 为什么 getIdentifier() 返回 0

当皮肤包构建时如果没有正确设置 Package ID，运行时 `getIdentifier()` 查找资源时会因为 Package ID 不匹配而失败，返回 0。

```java
// Android 内部查找逻辑（简化）
int getIdentifier(String name, String type, String packageName) {
    Package pkg = findPackage(packageName);
    if (pkg.packageId != 0x7f) {  // Package ID 不匹配
        return 0;  // 查找失败
    }
    return pkg.findResource(name, type);
}
```

## 解决方案

### ASB 已经提供了解决方案！

**好消息：** ASB 从 **2.0.0 版本**开始已经支持配置 Package ID，**默认值就是 0x7f**！

### 方案 1: 使用 ASB 默认配置（推荐）

ASB 2.0.0+ 默认使用 `0x7f` 作为 Package ID，直接使用即可：

```json
{
  "resourceDir": "./skin/res",
  "manifestPath": "./skin/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar"
}
```

ASB 会自动添加 `packageId: "0x7f"`。

### 方案 2: 明确指定 Package ID

在配置文件中明确指定（推荐用于重要项目）：

```json
{
  "resourceDir": "./skin/res",
  "manifestPath": "./skin/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "packageId": "0x7f"
}
```

### 方案 3: 使用命令行参数

```bash
asb build --package-id 0x7f --config asb.config.json
```

## Android 代码正确加载方法

除了正确构建皮肤包，还需要在 Android 代码中正确加载：

```java
public boolean loadSkin(String skinPath, String skinPackageName) {
    try {
        // 1. 创建 AssetManager
        AssetManager assetManager = AssetManager.class.newInstance();
        Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
        
        // 2. 添加皮肤包 - 检查返回值
        int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
        if (cookie == 0) {
            Log.e("SkinLoader", "Failed to load skin package");
            return false;  // 加载失败
        }
        
        // 3. 创建 Resources 对象
        Resources skinResources = new Resources(
            assetManager,
            context.getResources().getDisplayMetrics(),
            context.getResources().getConfiguration()
        );
        
        // 4. 获取资源 - 注意使用皮肤包的包名！
        int colorId = skinResources.getIdentifier(
            "primary_color",   // 资源名
            "color",           // 资源类型
            skinPackageName    // 皮肤包名（不是 app 包名！）
        );
        
        if (colorId == 0) {
            Log.e("SkinLoader", "Resource not found");
            return false;
        }
        
        // 5. 使用资源
        int color = skinResources.getColor(colorId, null);
        return true;
        
    } catch (Exception e) {
        e.printStackTrace();
        return false;
    }
}
```

### 关键点：

1. **检查 cookie 值**：`addAssetPath()` 返回 0 表示加载失败
2. **使用正确的包名**：必须使用皮肤包的 `packageName`，而不是应用的包名
3. **Package ID 必须是 0x7f**：这是标准 Android 应用的 Package ID

## 验证方法

### 1. 使用 aapt 验证 Package ID

```bash
# 构建皮肤包
asb build --config asb.config.json

# 验证 Package ID
aapt dump resources build/skin/com.example.skin.skin | grep "id=0x"
```

**预期输出：**
```
Package Group 0 id=0x7f packageCount=1 name=com.example.skin
  Package 0 id=0x7f name=com.example.skin
```

确认 `id=0x7f` 存在。

### 2. 在代码中验证

```java
int colorId = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
Log.d("SkinLoader", "Resource ID: 0x" + Integer.toHexString(colorId));

// 预期输出: Resource ID: 0x7f010000
//                           ^^  <- Package ID (0x7f)
```

如果看到 `0x7f` 开头的 ID，说明配置正确！

## 真机测试项目

我们已经创建了一个完整的 Android 测试项目，位于：

```
examples/android-skin-loader-test/
```

### 项目包含：

1. **皮肤包资源** (`skin/`)
   - colors.xml: 颜色定义
   - strings.xml: 字符串定义
   - AndroidManifest.xml: 皮肤包清单

2. **Android 测试应用** (`app/`)
   - MainActivity.java: 主界面
   - SkinLoader.java: 皮肤加载器（展示正确的加载方法）
   - 完整的 UI 和日志输出

3. **文档**
   - README.md: 项目说明
   - ANALYSIS.md: 详细的问题分析
   - QUICKSTART.md: 快速开始指南

4. **自动化脚本**
   - build-and-test.sh: 一键构建和测试

### 快速测试：

```bash
cd examples/android-skin-loader-test

# 1. 构建皮肤包
../../target/release/asb build --config asb.config.json

# 2. 验证 Package ID
aapt dump resources build/skin/com.example.skin.skin | grep "id=0x7f"

# 3. 构建 Android 应用（需要 Android SDK）
cd app
./gradlew assembleDebug

# 4. 部署到设备
adb install app/build/outputs/apk/debug/app-debug.apk
adb push ../build/skin/com.example.skin.skin /sdcard/Download/

# 5. 运行应用
adb shell am start -n com.example.skintest/.MainActivity

# 6. 在应用中点击 "Load Skin" 按钮测试
```

### 测试结果：

应用会显示：
- ✓ 皮肤包文件大小和路径
- ✓ Cookie 值（应该大于 0）
- ✓ 资源 ID（应该以 0x7f 开头）
- ✓ 颜色预览（应该变为皮肤颜色）
- ✓ 详细的日志输出

## 常见错误及解决

### 错误 1: getIdentifier() 返回 0

**原因**: Package ID 未设置为 0x7f

**解决**: 在 `asb.config.json` 中添加：
```json
{
  "packageId": "0x7f"
}
```

### 错误 2: 使用了错误的包名

```java
// ❌ 错误：使用了应用包名
getIdentifier("primary_color", "color", "com.example.myapp")

// ✓ 正确：使用皮肤包名
getIdentifier("primary_color", "color", "com.example.skin")
```

### 错误 3: addAssetPath 返回 0

**原因**: 文件不存在或无权限

**解决**:
```bash
# 检查文件
adb shell ls -l /sdcard/Download/com.example.skin.skin

# 修复权限
adb shell chmod 644 /sdcard/Download/com.example.skin.skin
```

## 技术实现细节

ASB 在 `src/aapt2.rs` 中实现了 Package ID 支持：

```rust
pub const DEFAULT_PACKAGE_ID: &str = "0x7f";

pub fn link(..., package_id: Option<&str>, ...) -> Result<LinkResult> {
    // ...
    let pkg_id = package_id.unwrap_or(DEFAULT_PACKAGE_ID);
    cmd.arg("--package-id").arg(pkg_id);
    // ...
}
```

这确保了每次构建都会传递正确的 Package ID 给 aapt2。

## 总结

**问题根源**: Android 资源系统依赖 Package ID 来识别和查找资源。如果 Package ID 不正确（不是 0x7f），`getIdentifier()` 无法找到资源，返回 0。

**解决方案**: 
1. **ASB 端**: 使用 ASB 2.0.0+ 版本，默认已经设置 `packageId = "0x7f"`
2. **Android 端**: 正确使用 AssetManager 和 Resources API，使用皮肤包的包名

**验证方法**: 
- 使用 `aapt dump resources` 验证 Package ID
- 检查 `getIdentifier()` 返回的资源 ID 是否以 0x7f 开头
- 使用测试项目进行真机验证

**完整示例**: 查看 `examples/android-skin-loader-test/` 目录

通过正确配置 ASB 和正确编写 Android 加载代码，可以完全解决 `getIdentifier()` 返回 0 的问题。
