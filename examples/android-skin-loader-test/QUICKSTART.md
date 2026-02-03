# Quick Start Guide - Android Skin Loader Test

## 🚀 快速开始

### 1. 构建皮肤包 (5 秒)

```bash
cd examples/android-skin-loader-test
../../target/release/asb build --config asb.config.json
```

**输出**: `build/skin/com.example.skin.skin` (Package ID: 0x7f ✓)

### 2. 验证 Package ID

```bash
aapt dump resources build/skin/com.example.skin.skin | grep "id=0x7f"
```

**期望输出**: `Package Group 0 id=0x7f` ✓

### 3. 测试应用

#### 选项 A: 使用自动化脚本

```bash
./build-and-test.sh
```

#### 选项 B: 手动步骤

```bash
# 1. 构建应用 (如果有 Android SDK)
cd app
./gradlew assembleDebug

# 2. 安装到设备
adb install app/build/outputs/apk/debug/app-debug.apk

# 3. 推送皮肤包
adb push build/skin/com.example.skin.skin /sdcard/Download/

# 4. 启动应用
adb shell am start -n com.example.skintest/.MainActivity
```

### 4. 在应用中测试

1. 点击 "Load Skin" 按钮
2. 查看日志输出
3. 验证：
   - ✓ Resource ID 不为 0 (例如: 0x7f010000)
   - ✓ 颜色预览改变
   - ✓ 状态显示 "Skin loaded successfully!"

## 🔍 问题排查

### getIdentifier() 返回 0？

**检查清单:**
1. `asb.config.json` 中有 `"packageId": "0x7f"` ✓
2. 使用 `aapt dump` 验证 Package ID ✓
3. 代码中包名是 `"com.example.skin"` (不是 app 包名) ✓
4. `addAssetPath()` 返回值非 0 ✓

### 皮肤包加载失败？

```bash
# 检查文件
adb shell ls -l /sdcard/Download/com.example.skin.skin

# 修复权限
adb shell chmod 644 /sdcard/Download/com.example.skin.skin
```

## 📖 详细文档

- [README.md](./README.md) - 完整项目说明
- [ANALYSIS.md](./ANALYSIS.md) - 问题根本原因分析

## 💡 核心要点

### ASB 配置

```json
{
  "packageId": "0x7f",  // ← 关键！
  "packageName": "com.example.skin"
}
```

### Android 代码

```java
// 关键 1: 检查 cookie
int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
if (cookie == 0) return false;  // 失败

// 关键 2: 使用皮肤包名
int id = skinResources.getIdentifier(
    "primary_color",    // 资源名
    "color",            // 资源类型
    "com.example.skin"  // ← 皮肤包名，不是 app 包名！
);
```

## 🎯 预期结果

成功加载后，日志应该显示：

```
Skin file exists: 12345 bytes
Successfully added asset path: /sdcard/Download/com.example.skin.skin
Cookie value: 2
Skin Resources created successfully
Test resource ID (primary_color): 0x7f010004
Applied primary_color to preview: 0xff6200ee
```

**关键检查点:**
- Cookie > 0 ✓
- Resource ID 以 0x7f 开头 ✓
- 颜色值正确 ✓

## 🐛 常见错误

| 错误 | 原因 | 解决方法 |
|------|------|----------|
| Resource ID = 0 | Package ID 未设置 | 设置 `"packageId": "0x7f"` |
| Cookie = 0 | 文件不存在/无权限 | 检查路径和权限 |
| 包名错误 | 使用了 app 包名 | 使用皮肤包名 |

## 📱 需要帮助？

查看应用内的日志输出，它会显示详细的调试信息！
