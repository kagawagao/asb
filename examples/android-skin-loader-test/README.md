# Android Skin Loader Test Project

这是一个完整的 Android 测试项目，用于演示如何正确加载和使用 ASB 生成的皮肤包。

## 问题分析：Resources.getIdentifier() 返回 0

### 问题描述

当通过 `new Resources()` API 动态加载皮肤包时，调用 `Resources.getIdentifier()` 方法返回 `0`，导致资源无法正常访问。

### 根本原因

这个问题的根本原因在于 Android 资源系统的工作机制：

1. **资源 ID 格式**：Android 资源 ID 的格式为 `0xPPTTEEEE`
   - `PP` = Package ID（包标识符）
   - `TT` = Type ID（类型标识符，如 color、string、drawable）
   - `EEEE` = Entry ID（条目标识符）

2. **Package ID 的重要性**：
   - 标准 Android 应用的 Package ID 是 `0x7f`
   - 如果打包时没有正确设置 Package ID，生成的资源 ID 可能使用默认值（如 `0x01`）
   - 通过 `Resources` API 动态加载时，如果 Package ID 不匹配，`getIdentifier()` 将无法找到资源

3. **ASB 的解决方案**：
   - 从版本 2.0.0 开始，ASB 支持通过 `packageId` 参数配置 Package ID
   - 默认值为 `0x7f`，这确保了皮肤包可以通过 `new Resources()` 正常加载

### 解决方案

#### 方案 1：在 ASB 配置中设置 packageId（推荐）

在 `asb.config.json` 中添加 `packageId` 字段：

```json
{
  "resourceDir": "./skin/res",
  "manifestPath": "./skin/AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "packageId": "0x7f",
  "incremental": true
}
```

#### 方案 2：使用命令行参数

```bash
asb build --package-id 0x7f --config asb.config.json
```

#### 方案 3：在 Android 代码中正确加载资源

即使设置了正确的 Package ID，也需要在 Android 代码中正确创建 Resources 对象：

```java
// 正确的方式
try {
    // 1. 创建 AssetManager
    AssetManager assetManager = AssetManager.class.newInstance();
    Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
    
    // 2. 添加皮肤包路径
    int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
    if (cookie == 0) {
        throw new RuntimeException("Failed to load skin package");
    }
    
    // 3. 创建 Resources 对象
    Resources skinResources = new Resources(
        assetManager,
        context.getResources().getDisplayMetrics(),
        context.getResources().getConfiguration()
    );
    
    // 4. 获取资源 ID
    // 注意：必须使用皮肤包的包名
    int colorId = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
    
    if (colorId != 0) {
        int color = skinResources.getColor(colorId, null);
        // 使用颜色
    }
    
} catch (Exception e) {
    e.printStackTrace();
}
```

**关键点：**
1. 使用反射调用 `AssetManager.addAssetPath()` 添加皮肤包
2. 检查返回的 cookie 是否非 0（0 表示加载失败）
3. 在 `getIdentifier()` 中使用皮肤包的正确包名（不是应用的包名）
4. 确保皮肤包文件具有正确的读取权限

### 常见错误

#### 错误 1：Package ID 未设置或错误

```bash
# 错误：没有设置 packageId（使用了默认的非标准值）
asb build --config asb.config.json

# 正确：明确设置为 0x7f
asb build --package-id 0x7f --config asb.config.json
```

#### 错误 2：使用错误的包名

```java
// 错误：使用应用的包名
int id = skinResources.getIdentifier("primary_color", "color", "com.example.app");

// 正确：使用皮肤包的包名
int id = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
```

#### 错误 3：没有检查 AssetManager.addAssetPath() 的返回值

```java
// 错误：没有检查返回值
Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
addAssetPath.invoke(assetManager, skinPath);

// 正确：检查返回值
int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
if (cookie == 0) {
    throw new RuntimeException("Failed to load skin package");
}
```

## 项目结构

```
android-skin-loader-test/
├── README.md                          # 本文件
├── skin/                              # 皮肤包资源
│   ├── res/
│   │   └── values/
│   │       ├── colors.xml             # 颜色定义
│   │       └── strings.xml            # 字符串定义
│   └── AndroidManifest.xml            # 皮肤包 Manifest
├── asb.config.json                    # ASB 配置文件
├── app/                               # Android 应用
│   ├── build.gradle                   # 应用构建脚本
│   ├── src/main/
│   │   ├── AndroidManifest.xml        # 应用 Manifest
│   │   ├── res/
│   │   │   ├── values/
│   │   │   │   ├── colors.xml         # 默认颜色
│   │   │   │   └── strings.xml        # 字符串
│   │   │   └── layout/
│   │   │       └── activity_main.xml  # 主界面布局
│   │   └── java/com/example/skintest/
│   │       ├── MainActivity.java      # 主活动
│   │       └── SkinLoader.java        # 皮肤加载器
├── build.gradle                       # 项目构建脚本
├── settings.gradle                    # 项目设置
└── build/                             # 构建输出（生成）
    └── skin/
        └── com.example.skin.skin      # 生成的皮肤包
```

## 构建和测试步骤

### 1. 构建皮肤包

```bash
cd examples/android-skin-loader-test

# 使用 ASB 构建皮肤包
../../target/release/asb build --config asb.config.json
```

这将在 `build/skin/` 目录下生成 `com.example.skin.skin` 文件。

### 2. 构建 Android 应用

```bash
# 如果使用 Gradle
cd app
./gradlew assembleDebug

# 或使用 Android Studio
# 直接在 Android Studio 中打开项目并构建
```

### 3. 部署到设备

```bash
# 1. 安装 APK
adb install app/build/outputs/apk/debug/app-debug.apk

# 2. 推送皮肤包到设备
adb push build/skin/com.example.skin.skin /sdcard/Download/

# 3. 启动应用
adb shell am start -n com.example.skintest/.MainActivity
```

### 4. 测试功能

应用应该能够：
1. 检测到皮肤包文件
2. 成功加载皮肤包
3. 通过 `getIdentifier()` 获取资源 ID（非 0）
4. 正确应用皮肤颜色和资源

## 验证资源 ID

### 使用 aapt 检查皮肤包

```bash
# 查看皮肤包中的资源
aapt dump resources build/skin/com.example.skin.skin

# 应该看到类似输出：
# Package Groups (1)
# Package Group 0 id=0x7f packageCount=1 name=com.example.skin
#   Package 0 id=0x7f name=com.example.skin
#     type 1 configCount=1 entryCount=2
#       spec resource 0x7f010000 com.example.skin:color/primary_color: flags=0x00000000
```

注意 Package ID 应该是 `0x7f`。

### 在代码中验证

添加日志输出：

```java
int colorId = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
Log.d("SkinLoader", "Resource ID: 0x" + Integer.toHexString(colorId));
// 应该输出类似：Resource ID: 0x7f010000
```

## 故障排除

### 问题：getIdentifier() 仍然返回 0

**检查清单：**
1. ✓ 确认 ASB 配置中设置了 `"packageId": "0x7f"`
2. ✓ 重新构建皮肤包
3. ✓ 使用 `aapt dump resources` 验证 Package ID
4. ✓ 确认代码中使用的是皮肤包的包名，而不是应用包名
5. ✓ 确认 AssetManager.addAssetPath() 返回值非 0

### 问题：AssetManager.addAssetPath() 返回 0

**可能原因：**
1. 皮肤包文件路径不正确
2. 皮肤包文件没有读取权限
3. 皮肤包文件已损坏

**解决方法：**
```bash
# 检查文件是否存在
adb shell ls -l /sdcard/Download/com.example.skin.skin

# 修复权限
adb shell chmod 644 /sdcard/Download/com.example.skin.skin
```

### 问题：Resources.getColor() 抛出异常

确保在 API 23+ 上使用正确的方法：

```java
// API 23+
int color = skinResources.getColor(colorId, null);

// API < 23
int color = skinResources.getColor(colorId);
```

## 参考资料

- [ASB 主文档](../../README.md)
- [Android Resources 官方文档](https://developer.android.com/reference/android/content/res/Resources)
- [Android 资源 ID 格式](https://developer.android.com/guide/topics/resources/providing-resources)
- [AAPT2 文档](https://developer.android.com/tools/aapt2)

## 总结

通过正确设置 ASB 的 `packageId` 为 `0x7f`，并在 Android 代码中正确使用 AssetManager 和 Resources API，可以完全解决 `getIdentifier()` 返回 0 的问题。关键是理解 Android 资源系统的 Package ID 机制，并确保皮肤包和加载代码都使用正确的配置。
