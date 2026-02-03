# ASB - Android Skin Builder

一个基于 aapt2 的高性能 Android 应用皮肤包打包工具 / A high-performance aapt2-based Android skin package builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/kagawagao/asb/workflows/CI/badge.svg)](https://github.com/kagawagao/asb/actions/workflows/ci.yml)

## Features / 特性

- 🎨 **资源打包** - 仅打包资源文件，支持热更新和插件化
- 📦 **AAR 支持** - 自动提取和打包依赖 AAR 包中的资源
- 🚀 **增量构建** - 支持增量打包，提升构建速度
- ⚡ **并发编译** - 充分利用 CPU 多核性能，支持并行资源编译
- 🔒 **资源 ID 稳定** - 支持 stable IDs，确保每次编译的资源 ID 不变
- 🔧 **脚本化工具** - 完全可通过命令行或配置文件使用
- 🌐 **跨平台** - 支持 Windows、macOS、Linux
- 💪 **Rust 实现** - 使用 Rust 编写，极致性能和内存安全

## Installation / 安装

### 从源码编译

```bash
git clone https://github.com/kagawagao/asb.git
cd asb
cargo build --release
# 二进制文件位于 target/release/asb
```

### 添加到 PATH

```bash
# Linux/macOS
sudo cp target/release/asb /usr/local/bin/

# Windows
# 将 target\release\asb.exe 复制到 PATH 中的目录
```

## Prerequisites / 前置条件

1. **Android SDK**: 需要安装 Android SDK 并配置 `ANDROID_HOME` 环境变量
2. **aapt2**: 工具会自动在 Android SDK 中查找 aapt2，或者可以手动指定路径
3. **Rust** (仅构建时需要): 1.70+ 版本

## Quick Start / 快速开始

### Option 1: 使用默认配置（推荐）

ASB 内置了基于标准 Android 项目结构的默认配置，无需配置文件即可直接使用：

```bash
# 在标准 Android 项目目录中直接运行
asb build
```

默认配置使用以下标准路径：
- 资源目录: `./src/main/res`
- Manifest: `./src/main/AndroidManifest.xml`
- 输出目录: `./build/outputs/skin`

### Option 2: 使用配置文件

#### 方法 A: 当前目录下的 asb.config.json（自动加载）

在项目根目录创建 `asb.config.json`，运行 `asb build` 时会自动使用：

```bash
# 生成默认配置文件
asb init

# 编辑 asb.config.json 后直接运行
asb build
```

生成的配置文件示例（基于标准 Android 结构）：

```json
{
  "resourceDir": "./src/main/res",
  "manifestPath": "./src/main/AndroidManifest.xml",
  "outputDir": "./build/outputs/skin",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-30/android.jar",
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0"
}
```

#### 方法 B: 指定配置文件路径

```bash
asb build --config custom-config.json
```

### Option 3: 命令行参数（最高优先级）

命令行参数会覆盖配置文件中的设置：

```bash
asb build \
  --resource-dir ./res \
  --manifest ./AndroidManifest.xml \
  --output ./build \
  --package com.example.skin \
  --android-jar $ANDROID_HOME/platforms/android-30/android.jar \
  --incremental \
  --workers 8
```

### 配置优先级

ASB 按以下优先级加载配置：

1. **命令行参数**（最高优先级）- 覆盖所有其他配置
2. **--config 指定的文件** - 显式指定的配置文件
3. **./asb.config.json** - 当前目录的配置文件（自动检测）
4. **内置默认配置**（最低优先级）- 基于标准 Android 项目结构

### 准备资源文件

标准 Android 项目结构（推荐）：

```
project/
├── src/
│   └── main/
│       ├── res/
│       │   ├── values/
│       │   │   └── colors.xml
│       │   ├── drawable/
│       │   │   └── icon.png
│       │   └── layout/
│       │       └── activity_main.xml
│       └── AndroidManifest.xml
└── asb.config.json (可选)
```

或传统结构：

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
└── asb.config.json (可选)
```

最小化的 `AndroidManifest.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.skin">
    <application />
</manifest>
```

## Usage / 使用方法

### Commands / 命令

#### `asb build`

构建皮肤包

**Options:**
- `-c, --config <path>` - 配置文件路径（可选，默认查找 ./asb.config.json）
- `-r, --resource-dir <path>` - 资源目录路径（覆盖配置文件）
- `-m, --manifest <path>` - AndroidManifest.xml 路径（覆盖配置文件）
- `-o, --output <path>` - 输出目录（覆盖配置文件）
- `-p, --package <name>` - 包名（覆盖配置文件）
- `-a, --android-jar <path>` - android.jar 路径（覆盖配置文件）
- `--aar <paths...>` - AAR 文件路径（可多个）
- `--aapt2 <path>` - aapt2 二进制文件路径
- `--incremental` - 启用增量构建
- `--version-code <number>` - 版本号
- `--version-name <string>` - 版本名称
- `--stable-ids <path>` - stable IDs 文件路径
- `--workers <number>` - 并行工作线程数（默认为 CPU 核心数）
- `--package-id <id>` - 资源包 ID（如 "0x7f"），用于动态资源加载

**说明:**
- 所有参数都是可选的
- 如果不提供 `--config`，工具会自动查找当前目录的 `./asb.config.json`
- 如果没有找到配置文件，会使用内置的默认配置（基于标准 Android 项目结构）
- 命令行参数始终优先于配置文件中的设置

**Examples:**

最简单的使用方式（标准 Android 项目）：

```bash
asb build
```

使用特定配置文件：

```bash
asb build --config custom-config.json
```

并行编译（8 个工作线程）：

```bash
asb build --config asb.config.json --workers 8
```

使用 stable IDs 保持资源 ID 稳定：

```bash
asb build --config asb.config.json --stable-ids ./stable-ids.txt
```

包含 AAR 依赖：

```bash
asb build \
  --config asb.config.json \
  --aar ./libs/library1.aar \
  --aar ./libs/library2.aar
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
  "compiledDir": "./build/compiled",
  "stableIdsFile": "./stable-ids.txt",
  "parallelWorkers": 8,
  "packageId": "0x7f"
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
| `stableIdsFile` | string | No | stable IDs 文件路径，用于保持资源 ID 稳定 |
| `parallelWorkers` | number | No | 并行工作线程数（默认为 CPU 核心数） |
| `packageId` | string | No | 资源包 ID（如 "0x7f"），用于动态资源加载（默认 "0x7f"） |

## Performance / 性能特性

### 并发编译

ASB 使用 Rust 的 Rayon 库实现并行资源编译：

- 默认使用所有可用 CPU 核心
- 可通过 `--workers` 参数或配置文件中的 `parallelWorkers` 自定义线程数
- 对于大型项目，并发编译可显著缩短构建时间

### 增量构建

- 使用 SHA-256 哈希检测文件变更
- 仅重新编译修改过的资源文件
- 缓存持久化到磁盘，重启后仍然有效

### Stable IDs

- 使用 aapt2 的 `--stable-ids` 和 `--emit-ids` 参数
- 确保每次编译生成的资源 ID 保持一致
- 对于热更新场景至关重要

### Package ID / 资源包 ID

**重要提示：** 从版本 2.0.0 起，ASB 支持配置 Package ID 来解决动态资源加载问题。

Android 资源 ID 格式为 `0xPPTTEEEE`，其中：
- `PP` = Package ID（包标识）
- `TT` = Type ID（类型标识，如 color、string）
- `EEEE` = Entry ID（条目标识）

**为什么需要设置 Package ID？**

当通过 Android 的 `new Resources()` API 动态加载皮肤包时，必须正确设置 Package ID，否则会导致所有资源 ID 无效（invalid resourceId）。

**默认值：**
- ASB 默认使用 `0x7f` 作为 Package ID（标准 Android 应用的 Package ID）
- 这确保皮肤包可以通过 `new Resources()` 正常加载

**自定义 Package ID：**

通过配置文件：
```json
{
  "packageId": "0x7f",
  ...
}
```

或通过命令行参数：
```bash
asb build --package-id 0x7f
```

**使用场景：**
- `0x7f`: 标准应用包（推荐用于动态加载）
- `0x7e`: 某些特殊插件化场景
- 其他值：根据具体插件化框架要求

## Use Cases / 使用场景

### 1. 应用皮肤/主题热更新

构建独立的资源包，通过热更新机制下发给用户：

```bash
asb build --config skin-theme.json --stable-ids stable-ids.txt
```

### 2. 大型项目快速构建

利用并发编译和增量构建加速开发：

```bash
asb build --config asb.config.json --incremental --workers 16
```

## Architecture / 架构

```
asb (Rust)
├── aapt2.rs       - aapt2 wrapper with parallel support
├── aar.rs         - AAR extraction
├── cache.rs       - Incremental build cache (SHA-256)
├── builder.rs     - Main build orchestration
├── merge.rs       - Internal merging utilities
├── cli.rs         - Command-line interface
├── types.rs       - Type definitions
└── main.rs        - Entry point
```

### 关键技术

- **Tokio**: 异步运行时
- **Rayon**: 数据并行处理
- **SHA2**: 文件哈希计算
- **Serde**: JSON 序列化
- **Clap**: 命令行解析

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

### 编译错误

检查资源文件格式是否正确，使用 `asb version` 确认 aapt2 可用。

### 并发问题

如果遇到并发相关问题，可以限制工作线程数：

```bash
asb build --config asb.config.json --workers 1
```

## Development / 开发

### 构建

```bash
cargo build --release
```

### 运行测试

```bash
cargo test
```

### 格式化代码

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

## License

MIT © Jingsong Gao

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [AAPT2 Documentation](https://developer.android.com/tools/aapt2)
- [Android Asset Packaging](https://android.googlesource.com/platform/frameworks/base/+/master/tools/aapt2/)
- [Rust Programming Language](https://www.rust-lang.org/)
