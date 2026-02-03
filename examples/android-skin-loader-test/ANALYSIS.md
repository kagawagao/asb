# Resources.getIdentifier() 返回 0 问题完整分析

## 问题重现

当使用 ASB 构建的皮肤包在 Android 运行时加载时，通过以下代码：

```java
AssetManager assetManager = AssetManager.class.newInstance();
Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
addAssetPath.invoke(assetManager, skinPath);

Resources skinResources = new Resources(
    assetManager,
    context.getResources().getDisplayMetrics(),
    context.getResources().getConfiguration()
);

// 问题：getIdentifier() 返回 0
int colorId = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
// colorId == 0
```

## 根本原因分析

### 1. Android 资源 ID 的结构

Android 资源 ID 是一个 32 位整数，格式为 `0xPPTTEEEE`：

```
0x 7f 01 0004
   │  │  └─ Entry ID (资源条目编号)
   │  └─── Type ID (资源类型: 01=color, 02=string, 等)
   └────── Package ID (包标识符)
```

**关键点：Package ID 决定了资源所属的包**

### 2. 标准 Package ID 值

- `0x01`: Android 系统资源 (android.R.xxx)
- `0x7f`: 标准 Android 应用资源 (R.xxx)
- `0x7e`, `0x7d`, ...: 某些插件化框架使用的自定义值

### 3. 问题产生机制

当 ASB 构建皮肤包时，如果没有明确指定 `packageId`，aapt2 可能会：

1. **使用默认值**（通常是 `0x7f`，但取决于 aapt2 版本）
2. **不设置 Package ID**，导致运行时无法正确识别

当 `getIdentifier()` 查询资源时：

```java
// Android 内部逻辑（简化版）
public int getIdentifier(String name, String defType, String defPackage) {
    // 1. 找到对应的 Package 对象
    Package pkg = findPackage(defPackage);
    if (pkg == null) return 0;  // 包不存在
    
    // 2. 在包中查找资源
    // 关键：需要通过 Package ID 来匹配
    if (pkg.packageId != expectedPackageId) {
        return 0;  // Package ID 不匹配
    }
    
    // 3. 查找资源名称
    int resourceId = pkg.findResource(name, defType);
    return resourceId;
}
```

**如果 Package ID 不正确，查找会失败，返回 0。**

### 4. ASB 2.0.0 的解决方案

ASB 从 2.0.0 版本开始，明确支持配置 `packageId`：

```rust
// src/aapt2.rs
pub const DEFAULT_PACKAGE_ID: &str = "0x7f";

// 在 link 函数中
let pkg_id = package_id.unwrap_or(DEFAULT_PACKAGE_ID);
cmd.arg("--package-id").arg(pkg_id);
```

这确保了：
1. **默认值为 `0x7f`**：符合标准 Android 应用的 Package ID
2. **可配置**：支持特殊场景（如插件化框架）
3. **传递给 aapt2**：通过 `--package-id` 参数明确指定

## 验证方法

### 方法 1：使用 aapt 检查

```bash
aapt dump resources your-skin-package.skin

# 查看输出中的 Package ID
# 正确输出应该是：
# Package Group 0 id=0x7f packageCount=1 name=com.example.skin
#   Package 0 id=0x7f name=com.example.skin
```

**关键检查点**：
- `id=0x7f` 必须存在
- Package name 必须与代码中使用的一致

### 方法 2：在代码中验证

```java
// 加载皮肤后，验证资源 ID
int colorId = skinResources.getIdentifier("primary_color", "color", "com.example.skin");

Log.d("SkinLoader", "Resource ID: 0x" + Integer.toHexString(colorId));

// 正确的输出应该是：
// Resource ID: 0x7f010000
//              ^^  <- Package ID (0x7f)
//                ^^ <- Type ID (01 = color)
//                  ^^^^ <- Entry ID
```

### 方法 3：使用 ASB 配置验证

```json
{
  "resourceDir": "./skin/res",
  "manifestPath": "./skin/AndroidManifest.xml",
  "packageName": "com.example.skin",
  "packageId": "0x7f",  // 确保这一行存在
  "outputDir": "./build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar"
}
```

## 完整解决方案

### 步骤 1：配置 ASB

确保 `asb.config.json` 中设置了正确的 `packageId`：

```json
{
  "packageId": "0x7f"
}
```

### 步骤 2：重新构建皮肤包

```bash
asb build --config asb.config.json
```

### 步骤 3：验证 Package ID

```bash
aapt dump resources build/com.example.skin.skin | grep "id=0x"
```

应该看到：`id=0x7f`

### 步骤 4：正确加载皮肤

```java
public boolean loadSkin(String skinPath, String skinPackageName) {
    try {
        // 1. 创建 AssetManager
        AssetManager assetManager = AssetManager.class.newInstance();
        Method addAssetPath = AssetManager.class.getMethod("addAssetPath", String.class);
        
        // 2. 添加皮肤包
        int cookie = (int) addAssetPath.invoke(assetManager, skinPath);
        if (cookie == 0) {
            // 加载失败
            return false;
        }
        
        // 3. 创建 Resources
        Resources skinResources = new Resources(
            assetManager,
            context.getResources().getDisplayMetrics(),
            context.getResources().getConfiguration()
        );
        
        // 4. 获取资源 - 使用皮肤包的包名
        int colorId = skinResources.getIdentifier(
            "primary_color",  // 资源名
            "color",          // 资源类型
            skinPackageName   // 必须是皮肤包的包名！
        );
        
        if (colorId != 0) {
            // 成功！
            int color = skinResources.getColor(colorId, null);
            return true;
        }
        
        return false;
        
    } catch (Exception e) {
        e.printStackTrace();
        return false;
    }
}
```

## 常见错误及修复

### 错误 1：未设置 packageId

**症状**：`getIdentifier()` 始终返回 0

**原因**：ASB 配置中没有 `packageId` 字段，或者使用了错误的值

**修复**：
```json
{
  "packageId": "0x7f"
}
```

### 错误 2：使用了错误的包名

**症状**：`getIdentifier()` 返回 0，但 aapt 显示 Package ID 正确

**原因**：代码中使用的包名与皮肤包的实际包名不匹配

```java
// 错误
int id = skinResources.getIdentifier("color", "primary_color", "com.example.app");

// 正确
int id = skinResources.getIdentifier("primary_color", "color", "com.example.skin");
```

**修复**：确保包名与 ASB 配置中的 `packageName` 一致

### 错误 3：addAssetPath 返回 0

**症状**：AssetManager.addAssetPath() 返回 0

**原因**：
1. 皮肤包文件不存在或路径错误
2. 文件权限不足
3. 皮肤包文件损坏

**修复**：
```bash
# 检查文件是否存在
adb shell ls -l /sdcard/Download/your-skin.skin

# 修复权限
adb shell chmod 644 /sdcard/Download/your-skin.skin

# 重新推送文件
adb push your-skin.skin /sdcard/Download/
```

### 错误 4：参数顺序错误

**症状**：编译通过但 getIdentifier 返回 0

**原因**：getIdentifier 参数顺序错误

```java
// 错误的参数顺序
getIdentifier("color", "primary_color", "com.example.skin")
//            ^^^^^^   ^^^^^^^^^^^^^
//            类型      名称  <- 顺序错了！

// 正确的参数顺序
getIdentifier("primary_color", "color", "com.example.skin")
//            ^^^^^^^^^^^^^   ^^^^^^
//            名称            类型
```

## 测试检查清单

使用以下检查清单确保配置正确：

- [ ] ASB 配置中设置了 `"packageId": "0x7f"`
- [ ] 使用 `aapt dump resources` 验证 Package ID 是 0x7f
- [ ] 代码中使用的包名与 ASB 配置的 `packageName` 一致
- [ ] `getIdentifier()` 参数顺序正确：(name, type, package)
- [ ] AssetManager.addAssetPath() 返回值非 0
- [ ] 皮肤包文件存在且有读取权限
- [ ] 资源名称在皮肤包中确实存在

## 技术细节

### aapt2 的 --package-id 参数

aapt2 link 命令支持 `--package-id` 参数：

```bash
aapt2 link \
  --manifest AndroidManifest.xml \
  -I android.jar \
  --package-id 0x7f \  # 指定 Package ID
  -o output.apk \
  *.flat
```

ASB 在 `src/aapt2.rs` 的 `link` 函数中使用这个参数：

```rust
cmd.arg("--package-id").arg(pkg_id);
```

### Resources.getIdentifier() 的内部实现

Android 系统通过以下步骤查找资源：

1. **解析资源 ID 格式**：
   ```
   0x7f010004
     ^^  <- Package ID (用于定位包)
       ^^ <- Type ID (用于定位资源类型)
         ^^^^ <- Entry ID (用于定位具体资源)
   ```

2. **根据 Package ID 查找包**：
   ```java
   Package pkg = mPackages[packageId];
   ```

3. **在包中查找类型和条目**：
   ```java
   Type type = pkg.types[typeId];
   Entry entry = type.entries[entryId];
   ```

4. **返回资源 ID**：
   ```java
   return (packageId << 24) | (typeId << 16) | entryId;
   ```

如果任何一步失败，返回 0。

## 参考资料

- [Android Resources 文档](https://developer.android.com/reference/android/content/res/Resources)
- [AAPT2 文档](https://developer.android.com/tools/aapt2)
- [ASB README](../../README.md)
- [完整测试项目](./README.md)

## 总结

`Resources.getIdentifier()` 返回 0 的问题主要由以下原因引起：

1. **Package ID 未正确设置**：ASB 默认使用 0x7f，但需要确保配置正确
2. **包名不匹配**：代码中使用的包名必须与皮肤包的实际包名一致
3. **加载失败**：AssetManager.addAssetPath() 返回 0
4. **参数错误**：getIdentifier() 的参数顺序或内容错误

通过正确配置 ASB 的 `packageId` 为 `0x7f`，并在代码中正确使用 AssetManager 和 Resources API，可以完全解决这个问题。本测试项目提供了完整的示例代码和验证方法。
