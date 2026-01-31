# ASB - Android Skin Builder

一个基于 aapt2 的 Android 应用皮肤包打包工具 / An aapt2-based Android skin package builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features / 特性

- 🎨 **资源打包** - 仅打包资源文件，支持热更新和插件化
- 📦 **AAR 支持** - 自动提取和打包依赖 AAR 包中的资源
- 🚀 **增量构建** - 支持增量打包，提升构建速度
- 🔧 **脚本化工具** - 完全可通过命令行或配置文件使用
- 🌐 **跨平台** - 支持 Windows、macOS、Linux
- ⚡ **直接调用 aapt2** - 使用 Android 官方工具，确保兼容性

## Installation / 安装

```bash
npm install -g asb
```

Or use locally in your project:

```bash
npm install --save-dev asb
```

## Prerequisites / 前置条件

1. **Android SDK**: 需要安装 Android SDK 并配置 `ANDROID_HOME` 环境变量
2. **aapt2**: 工具会自动在 Android SDK 中查找 aapt2，或者可以手动指定路径

## Quick Start / 快速开始

### 1. 初始化项目

```bash
asb init
```

This creates a sample configuration file `asb.config.json`:

```json
{
  "resourceDir": "./res",
  "manifestPath": "./AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-30/android.jar",
  "aarFiles": [],
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0"
}
```

### 2. 准备资源文件

创建标准的 Android 资源结构：

```
project/
├── res/
│   ├── values/
│   │   └── colors.xml
│   ├── drawable/
│   │   └── icon.png
│   └── layout/
│       └── activity_main.xml
├── AndroidManifest.xml
└── asb.config.json
```

最小化的 `AndroidManifest.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.skin">
    <application />
</manifest>
```

### 3. 构建皮肤包

使用配置文件：

```bash
asb build --config asb.config.json
```

或直接使用命令行参数：

```bash
asb build \
  --resource-dir ./res \
  --manifest ./AndroidManifest.xml \
  --output ./build \
  --package com.example.skin \
  --android-jar $ANDROID_HOME/platforms/android-30/android.jar \
  --incremental
```

## Usage / 使用方法

### Commands / 命令

#### `asb build`

构建皮肤包

**Options:**
- `-c, --config <path>` - 配置文件路径
- `-r, --resource-dir <path>` - 资源目录路径
- `-m, --manifest <path>` - AndroidManifest.xml 路径
- `-o, --output <path>` - 输出目录
- `-p, --package <name>` - 包名
- `-a, --android-jar <path>` - android.jar 路径
- `--aar <paths...>` - AAR 文件路径（可多个）
- `--aapt2 <path>` - aapt2 二进制文件路径
- `--incremental` - 启用增量构建
- `--version-code <number>` - 版本号
- `--version-name <string>` - 版本名称

**Examples:**

包含 AAR 依赖：

```bash
asb build \
  --config asb.config.json \
  --aar ./libs/library1.aar \
  --aar ./libs/library2.aar
```

指定版本信息：

```bash
asb build \
  --config asb.config.json \
  --version-code 2 \
  --version-name "1.1.0"
```

#### `asb clean`

清理构建产物

```bash
asb clean --config asb.config.json
# or
asb clean --output ./build
```

#### `asb version`

显示 aapt2 版本

```bash
asb version
```

#### `asb init`

初始化项目配置

```bash
asb init
# or specify directory
asb init --dir ./my-skin-project
```

## Configuration / 配置

### Configuration File / 配置文件

完整的配置文件示例：

```json
{
  "resourceDir": "./res",
  "manifestPath": "./AndroidManifest.xml",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "/path/to/android.jar",
  "aarFiles": [
    "./libs/library1.aar",
    "./libs/library2.aar"
  ],
  "aapt2Path": "/path/to/aapt2",
  "incremental": true,
  "cacheDir": "./build/.cache",
  "versionCode": 1,
  "versionName": "1.0.0",
  "additionalResourceDirs": [
    "./extra-res"
  ],
  "compiledDir": "./build/compiled"
}
```

### Configuration Options / 配置选项

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `resourceDir` | string | Yes | 资源目录路径 |
| `manifestPath` | string | Yes | AndroidManifest.xml 路径 |
| `outputDir` | string | Yes | 输出目录 |
| `packageName` | string | Yes | 包名 |
| `androidJar` | string | Yes | android.jar 路径 |
| `aarFiles` | string[] | No | AAR 文件列表 |
| `aapt2Path` | string | No | aapt2 路径（自动检测） |
| `incremental` | boolean | No | 启用增量构建（默认 false） |
| `cacheDir` | string | No | 缓存目录（默认 outputDir/.build-cache） |
| `versionCode` | number | No | 版本号 |
| `versionName` | string | No | 版本名称 |
| `additionalResourceDirs` | string[] | No | 额外的资源目录 |
| `compiledDir` | string | No | 编译产物目录（默认 outputDir/compiled） |

## Use Cases / 使用场景

### 1. 应用皮肤/主题热更新

构建独立的资源包，通过热更新机制下发给用户：

```bash
asb build --config skin-theme.json --version-name "theme-dark-v1"
```

### 2. 插件化开发

为插件化应用构建资源包：

```bash
asb build \
  --resource-dir ./plugin-res \
  --manifest ./plugin-manifest.xml \
  --package com.example.plugin \
  --android-jar $ANDROID_HOME/platforms/android-30/android.jar \
  --output ./plugin-build
```

### 3. 多 AAR 依赖整合

整合多个 AAR 库的资源：

```bash
asb build \
  --config base.json \
  --aar ./libs/ui-lib.aar \
  --aar ./libs/theme-lib.aar \
  --aar ./libs/icons-lib.aar
```

### 4. CI/CD 集成

在持续集成中使用：

```yaml
# .github/workflows/build-skin.yml
name: Build Skin Package

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions/setup-node@v2
        with:
          node-version: '18'
      - name: Install ASB
        run: npm install -g asb
      - name: Build Skin
        run: asb build --config asb.config.json
      - name: Upload Artifact
        uses: actions/upload-artifact@v2
        with:
          name: skin-package
          path: build/*.apk
```

## Incremental Build / 增量构建

启用增量构建可以显著提升构建速度：

```bash
asb build --config asb.config.json --incremental
```

增量构建会：
- 缓存已编译的 .flat 文件
- 计算文件哈希值检测变更
- 只重新编译修改过的资源文件

首次构建后，只有修改的文件会被重新编译。

## Troubleshooting / 故障排除

### aapt2 not found

确保安装了 Android SDK 并设置了 `ANDROID_HOME` 环境变量：

```bash
export ANDROID_HOME=/path/to/android-sdk
```

或手动指定 aapt2 路径：

```bash
asb build --aapt2 /path/to/aapt2 ...
```

### Resource compilation errors

检查资源文件格式是否正确，使用 `asb version` 确认 aapt2 可用。

### AAR extraction errors

确保 AAR 文件存在且未损坏。

## API Usage / API 使用

也可以在代码中使用：

```typescript
import { SkinBuilder } from 'asb';

const builder = new SkinBuilder({
  resourceDir: './res',
  manifestPath: './AndroidManifest.xml',
  outputDir: './build',
  packageName: 'com.example.skin',
  androidJar: '/path/to/android.jar',
  incremental: true,
});

const result = await builder.build();

if (result.success) {
  console.log('Built:', result.apkPath);
} else {
  console.error('Errors:', result.errors);
}
```

## License

MIT © Jingsong Gao

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [AAPT2 Documentation](https://developer.android.com/tools/aapt2)
- [Android Asset Packaging](https://android.googlesource.com/platform/frameworks/base/+/master/tools/aapt2/)
