# ASB - Android Skin Builder

一个基于 aapt2 的高性能 Android 应用皮肤包打包工具 / A high-performance aapt2-based Android skin package builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features / 特性

- 🎨 **资源打包** - 仅打包资源文件，支持热更新和插件化
- 📦 **AAR 支持** - 自动提取和打包依赖 AAR 包中的资源
- 🚀 **增量构建** - 支持增量打包，提升构建速度
- ⚡ **并发编译** - 充分利用 CPU 多核性能，支持并行资源编译
- 🔒 **资源 ID 稳定** - 支持 stable IDs，确保每次编译的资源 ID 不变
- 🎯 **资源优先级** - 按照 Android 资源优先级策略处理资源冲突和覆盖
- 🔧 **脚本化工具** - 完全可通过命令行或配置文件使用
- 🌐 **跨平台** - 支持 Windows、macOS、Linux
- 💪 **Rust 实现** - 使用 Rust 编写，极致性能和内存安全

## Installation / 安装

### 下载预编译二进制文件（推荐）

从 [Releases 页面](https://github.com/kagawagao/asb/releases) 下载适合您系统的预编译二进制文件：

- **Linux**: `asb-linux-x86_64` (x64) 或 `asb-linux-aarch64` (ARM64)
- **Debian 11**: `asb-debian11-x86_64` (x64, 兼容旧版 glibc)
- **macOS**: `asb-macos-x86_64` (Intel) 或 `asb-macos-aarch64` (Apple Silicon)
- **Windows**: `asb-windows-x86_64.exe` (x64) 或 `asb-windows-aarch64.exe` (ARM64)

解压并添加到 PATH：

```bash
# Linux/macOS (示例: x86_64)
tar xzf asb-linux-x86_64.tar.gz
sudo mv asb-linux-x86_64 /usr/local/bin/asb
sudo chmod +x /usr/local/bin/asb

# Windows (示例: x64)
# 解压 asb-windows-x86_64.zip
# 将 asb-windows-x86_64.exe 移动到 PATH 中的目录
```

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

### Option 1: 使用配置文件（推荐）

#### 单应用配置

在项目根目录创建 `asb.config.json`：

```bash
# 生成默认配置文件
asb init

# 编辑 asb.config.json 后直接运行
asb build
```

**配置示例**：

```json
{
  "resourceDir": "./res",
  "outputDir": "./build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar", // 可选，未指定时自动检测最高版本
  "incremental": true
}
```

更多配置选项（多应用、Flavors、资源优先级等）请参见 [Configuration](#configuration--配置) 章节。

### Option 2: 命令行参数

命令行参数会覆盖配置文件中的设置：

```bash
asb build \
  --resource-dir ./res \
  --manifest ./AndroidManifest.xml \
  --output ./build \
  --package com.example.skin \
  --android-jar $ANDROID_HOME/platforms/android-34/android.jar \
  --incremental \
  --package-id 0x7f
```

**多应用配置的过滤**：

```bash
# 只构建特定包名的应用
asb build --config asb.config.json --packages com.example.skin.app1,com.example.skin.app2
```

### 配置优先级

ASB 按以下优先级加载配置：

1. **命令行参数**（最高优先级）- 覆盖所有其他配置
2. **--config 指定的文件** - 显式指定的配置文件
3. **./asb.config.json** - 当前目录的配置文件（自动检测）

### 项目结构

```
project/
├── res/
│   ├── values/
│   │   └── colors.xml
│   └── drawable/
│       └── icon.png
└── asb.config.json
```

**注意**：AndroidManifest.xml 可以省略，工具会自动生成。更多结构示例请参见 [Examples](#examples--示例) 章节。

## Usage / 使用方法

### Commands / 命令

#### `asb build`

构建皮肤包

**Options:**

- `-q, --quiet` - 静默模式（仅显示错误和结果信息，不显示过程日志）
- `-c, --config <path>` - 配置文件路径（可选，默认查找 ./asb.config.json）
- `-r, --resource-dir <path>` - 资源目录路径（覆盖配置文件）
- `-m, --manifest <path>` - AndroidManifest.xml 路径（可选，会自动生成）
- `-o, --output <path>` - 输出目录（覆盖配置文件）
- `-p, --package <name>` - 包名（覆盖配置文件）
- `-a, --android-jar <path>` - android.jar 路径（覆盖配置文件）
- `--aar <paths...>` - AAR 文件路径（可多个）
- `--aapt2 <path>` - aapt2 二进制文件路径
- `--incremental` - 启用增量构建
- `--version-code <number>` - 版本号
- `--version-name <string>` - 版本名称
- `--stable-ids <path>` - stable IDs 文件路径
- `--package-id <id>` - 资源包 ID（如 "0x7f"），用于动态资源加载
- `--max-parallel-builds <number>` - 多应用配置的最大并行构建数（默认为 CPU 核心数）
- `--packages <names...>` - 过滤要构建的包名（逗号分隔），仅构建匹配的配置

**说明:**

- 所有参数都是可选的
- 如果不提供 `--config`，工具会自动查找当前目录的 `./asb.config.json`
- 命令行参数始终优先于配置文件中的设置
- AndroidManifest.xml 可以省略，会自动生成最小化的 manifest

**Examples:**

最简单的使用方式（有配置文件）：

```bash
asb build
```

使用特定配置文件：

```bash
asb build --config custom-config.json
```

只构建特定包名的应用（多应用配置）：

```bash
asb build --config asb.config.json --packages com.example.skin.app1
```

控制并行构建数：

```bash
asb build --config asb.config.json --max-parallel-builds 4
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

完全使用命令行参数：

```bash
asb build \
  --resource-dir ./res \
  --output ./build \
  --package com.example.skin \
  --android-jar $ANDROID_HOME/platforms/android-34/android.jar \
  --incremental \
  --package-id 0x7f
```

使用静默模式（仅显示错误和结果）：

```bash
asb --quiet build --config asb.config.json
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

ASB 支持两种配置格式：**单应用配置** 和 **多应用配置**。

#### 单应用配置（Single App）

适用于构建单个应用的皮肤包：

```json
{
  "resourceDir": "./res",
  "manifestPath": "./AndroidManifest.xml",
  "outputDir": "./build",
  "buildDir": "./build/.build",
  "packageName": "com.example.skin",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "aarFiles": ["./libs/library1.aar"],
  "aapt2Path": "/path/to/aapt2",
  "incremental": true,
  "cacheDir": "./build/.cache",
  "versionCode": 1,
  "versionName": "1.0.0",
  "additionalResourceDirs": ["./extra-res"],
  "stableIdsFile": "./stable-ids.txt",
  "packageId": "0x7f"
}
```

#### 多应用配置（Multi-App）

适用于在一个配置文件中构建多个应用的皮肤包，公共配置在顶层定义：

```json
{
  "baseDir": "./",
  "outputDir": "./build",
  "buildDir": "./build/.build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "incremental": true,
  "versionCode": 1,
  "versionName": "1.0.0",
  "maxParallelBuilds": 4,
  "packageId": "0x7f",
  "apps": [
    {
      "baseDir": "./app1",
      "packageName": "com.example.skin.app1",
      "outputFile": "app1.skin"
    },
    {
      "baseDir": "./app2",
      "packageName": "com.example.skin.app2",
      "additionalResourceDirs": ["./app1/res"],
      "outputFile": "app2.skin"
    }
  ]
}
```

#### Flavors 配置（产品变体）

支持为同一应用构建多个变体（如 free/pro，debug/release）：

```json
{
  "outputDir": "./build",
  "androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar",
  "apps": [
    {
      "baseDir": "./",
      "packageName": "com.example.skin.myapp",
      "flavors": [
        {
          "name": "free",
          "outputFile": "myapp-free.skin",
          "additionalResourceDirs": ["./flavors/free/res"]
        },
        {
          "name": "pro",
          "outputFile": "myapp-pro.skin",
          "additionalResourceDirs": ["./flavors/pro/res"],
          "versionCode": 2
        }
      ]
    }
  ]
}
```

### Configuration Options / 配置选项

#### 单应用配置选项

| Option                   | Type     | Required | Description                                                                                            |
| ------------------------ | -------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `resourceDir`            | string   | Yes\*    | 资源目录路径（使用 baseDir 时可选）                                                                    |
| `manifestPath`           | string   | No       | AndroidManifest.xml 路径（可省略，自动生成）                                                           |
| `outputDir`              | string   | Yes      | 输出目录                                                                                               |
| `packageName`            | string   | Yes      | 包名                                                                                                   |
| `androidJar`             | string   | No       | android.jar 路径，支持 `${ANDROID_HOME}` 环境变量（可选，未指定时自动检测 platforms 目录下版本最高的） |
| `baseDir`                | string   | No       | 基础目录，自动推导 resourceDir 和 manifestPath                                                         |
| `aarFiles`               | string[] | No       | AAR 文件列表                                                                                           |
| `aapt2Path`              | string   | No       | aapt2 路径（自动检测）                                                                                 |
| `incremental`            | boolean  | No       | 启用增量构建（默认 false）                                                                             |
| `buildDir`               | string   | No       | 构建目录，用于存放中间文件和缓存（默认 `{outputDir}/.build`）                                          |
| `cacheDir`               | string   | No       | 缓存目录（已废弃，推荐使用 buildDir）                                                                  |
| `versionCode`            | number   | No       | 版本号                                                                                                 |
| `versionName`            | string   | No       | 版本名称                                                                                               |
| `additionalResourceDirs` | string[] | No       | 额外的资源目录（用于资源覆盖）                                                                         |
| `stableIdsFile`          | string   | No       | stable IDs 文件路径，用于保持资源 ID 稳定                                                              |
| `packageId`              | string   | No       | 资源包 ID（如 "0x7f"），用于动态资源加载（默认 "0x7f"）                                                |
| `outputFile`             | string   | No       | 自定义输出文件名（默认为 `{packageName}.skin`）                                                        |

#### 多应用配置选项

**顶层公共配置**：

| Option              | Type     | Required | Description                                                                |
| ------------------- | -------- | -------- | -------------------------------------------------------------------------- |
| `apps`              | array    | Yes      | 应用配置数组                                                               |
| `outputDir`         | string   | Yes      | 公共输出目录                                                               |
| `androidJar`        | string   | No       | 公共 android.jar 路径（可选，未指定时自动检测 platforms 目录下版本最高的） |
| `baseDir`           | string   | No       | 公共基础目录                                                               |
| `incremental`       | boolean  | No       | 公共增量构建设置                                                           |
| `buildDir`          | string   | No       | 公共构建目录，用于存放中间文件和缓存（默认 `{outputDir}/.build`）          |
| `cacheDir`          | string   | No       | 公共缓存目录（已废弃，推荐使用 buildDir）                                  |
| `versionCode`       | number   | No       | 公共版本号（可被应用级配置覆盖）                                           |
| `versionName`       | string   | No       | 公共版本名称（可被应用级配置覆盖）                                         |
| `packageId`         | string   | No       | 公共资源包 ID（可被应用级配置覆盖）                                        |
| `maxParallelBuilds` | number   | No       | 最大并行构建数（默认为 CPU 核心数）                                        |
| `aarFiles`          | string[] | No       | 公共 AAR 文件列表                                                          |
| `aapt2Path`         | string   | No       | 公共 aapt2 路径                                                            |
| `stableIdsFile`     | string   | No       | 公共 stable IDs 文件                                                       |

**应用级配置（apps 数组中的每个项）**：

| Option                   | Type     | Required | Description                       |
| ------------------------ | -------- | -------- | --------------------------------- |
| `packageName`            | string   | Yes      | 应用包名                          |
| `baseDir`                | string   | No       | 应用特定基础目录                  |
| `resourceDir`            | string   | No       | 应用特定资源目录                  |
| `manifestPath`           | string   | No       | 应用特定 manifest 路径            |
| `outputDir`              | string   | No       | 应用特定输出目录（覆盖公共配置）  |
| `outputFile`             | string   | No       | 应用特定输出文件名                |
| `additionalResourceDirs` | string[] | No       | 应用特定额外资源目录              |
| `versionCode`            | number   | No       | 应用特定版本号（覆盖公共配置）    |
| `versionName`            | string   | No       | 应用特定版本名称（覆盖公共配置）  |
| `packageId`              | string   | No       | 应用特定资源包 ID（覆盖公共配置） |
| `flavors`                | array    | No       | 应用的产品变体配置数组            |

**Flavor 配置选项**：

| Option                   | Type     | Required | Description             |
| ------------------------ | -------- | -------- | ----------------------- |
| `name`                   | string   | Yes      | Flavor 名称             |
| `outputFile`             | string   | No       | Flavor 特定输出文件名   |
| `additionalResourceDirs` | string[] | No       | Flavor 特定额外资源目录 |
| `versionCode`            | number   | No       | Flavor 特定版本号       |
| `versionName`            | string   | No       | Flavor 特定版本名称     |
| `packageId`              | string   | No       | Flavor 特定资源包 ID    |

### 配置说明

**baseDir 自动推导**：

- 当指定 `baseDir` 时，如果未指定 `resourceDir`，则默认为 `{baseDir}/res`
- 当指定 `baseDir` 时，如果未指定 `manifestPath`，则默认为 `{baseDir}/AndroidManifest.xml`
- 这简化了标准 Android 项目结构的配置

**manifestPath 可选**：

- 从 ASB 2.0 开始，AndroidManifest.xml 可以省略
- 工具会自动生成最小化的 manifest：`<manifest package="{packageName}" />`

**buildDir 构建目录**：

- `buildDir` 用于存放构建过程中的中间文件和缓存，与 `outputDir`（最终产物）区分开
- 默认值：`{outputDir}/.build`
- 中间文件包括：编译后的资源文件（.flat）、临时文件、构建缓存等
- **注意**：`cacheDir` 已废弃，推荐使用 `buildDir`。如果同时指定了两者，`cacheDir` 优先用于缓存

**环境变量支持**：

- 配置文件中的路径支持环境变量展开，如 `${ANDROID_HOME}`
- 示例：`"androidJar": "${ANDROID_HOME}/platforms/android-34/android.jar"`

**androidJar 自动检测**：

- `androidJar` 字段现在是可选的
- 如果未指定，工具会自动扫描 `$ANDROID_HOME/platforms` 目录
- 自动选择版本号最高的 `android.jar`（如 android-35、android-34 等）
- 确保已设置 `ANDROID_HOME` 环境变量以使用自动检测功能

## Performance / 性能特性

### 并发编译

ASB 实现了两层并发优化：

- **资源编译并发**：自动设置为 CPU 核心数的 2 倍，充分利用系统资源
- **多配置构建并发**：可通过 `--max-parallel-builds` 参数或配置文件中的 `maxParallelBuilds` 自定义最大并行数（默认为 CPU 核心数）
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

### Resource Priority / 资源优先级

**重要提示：** ASB 从版本 2.0.0 起，支持按照 Android 标准资源优先级策略处理资源冲突。

当多个资源目录包含同名资源时，ASB 会按照 **Android 标准优先级** 进行覆盖（数字越大优先级越高）：

1. **AAR 依赖资源** (`aarFiles`) - 最低优先级（Library Dependencies）
2. **主资源目录** (`resourceDir`) - 中等优先级（Main Source Set）
3. **额外资源目录** (`additionalResourceDirs`) - 最高优先级（Product Flavor / Build Type）

**符合 Android Gradle 构建标准：**

```
Library Dependencies < Main Source Set < Product Flavor < Build Type
```

**工作原理：**

ASB 使用 aapt2 的 `-R` 标志实现资源覆盖语义：

- AAR 依赖资源（如果存在）作为基础资源，否则主资源目录作为基础
- 其他资源作为覆盖层（overlay）链接
- 当存在同名资源时，优先级高的资源会覆盖优先级低的资源

**示例：**

```json
{
  "resourceDir": "./src/main/res",
  "aarFiles": ["./libs/theme-lib.aar"],
  "additionalResourceDirs": ["./src/free/res", "./src/debug/res"]
}
```

如果四个来源都定义了 `primary_color`：

- `theme-lib.aar`: `primary_color = #FF0000`（最低优先级 - Library）
- `src/main/res`: `primary_color = #00FF00`（Main）
- `src/free/res`: `primary_color = #0000FF`（Product Flavor）
- `src/debug/res`: `primary_color = #FFFF00`（最高优先级 - Build Type）

最终皮肤包中 `primary_color` 的值为 `#FFFF00`（来自 `src/debug/res` - Build Type）。

**完整示例：**

参见 `examples/resource-priority-test/` 目录，展示了资源优先级的完整用法。

## Examples / 示例

ASB 提供了多个示例项目来演示不同的使用场景和功能。详见 [examples/](examples/) 目录：

- **simple-skin** - 基础皮肤包示例
- **multi-theme** - 多主题支持（日/夜模式）
- **android-skin-loader-test** - 完整的 Android 应用示例，演示如何动态加载皮肤包
- **array-config** / **array-config-deps** - 多应用配置示例
- **resource-priority-test** - 资源优先级测试
- 更多示例请查看 [examples/README.md](examples/README.md)

## Use Cases / 使用场景

### 1. 应用皮肤/主题热更新

构建独立的资源包，通过热更新机制下发给用户：

```bash
asb build --config skin-theme.json --stable-ids stable-ids.txt
```

### 2. 大型项目快速构建

利用并发编译和增量构建加速开发：

```bash
asb build --config asb.config.json --incremental --max-parallel-builds 8
```

### 3. 多层资源覆盖

使用资源优先级特性实现主题定制，详见 [Resource Priority](#resource-priority--资源优先级) 章节。

## Architecture / 架构

```
asb (Rust)
├── aapt2.rs            - aapt2 wrapper with parallel support
├── aar.rs              - AAR extraction
├── cache.rs            - Incremental build cache (SHA-256)
├── builder.rs          - Main build orchestration
├── dependency.rs       - Multi-app dependency resolution
├── resource_priority.rs - Resource priority handling
├── merge.rs            - Internal merging utilities
├── cli.rs              - Command-line interface
├── types.rs            - Type definitions
├── lib.rs              - Library interface
└── main.rs             - Entry point
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

如果遇到并发相关问题，可以限制并行构建数：

```bash
asb build --config asb.config.json --max-parallel-builds 1
```

## Development

### Running Tests

```bash
cargo test --all-targets
```

### Code Quality

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

### CI/CD

CI runs on every PR:
- Quality gate: fmt, clippy, tests
- Build matrix: 7 platforms (Linux/Mac/Windows, x86_64/ARM64)
- Binary validation

### Project Structure

- `src/builder.rs` — Core build orchestrator
- `src/aapt2.rs` — aapt2 CLI wrapper
- `src/aar.rs` — AAR extraction
- `src/cache.rs` — Incremental build cache (SHA-256)
- `src/types.rs` — Configuration types
- `src/cli.rs` — CLI argument parsing
- `src/resource_priority.rs` — Android resource priority resolution
- `src/dependency.rs` — Dependency graph analysis
- `src/merge.rs` — Manifest merging

## License

MIT © Jingsong Gao

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For more details on development setup and guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Links

- [AAPT2 Documentation](https://developer.android.com/tools/aapt2)
- [Android Asset Packaging](https://android.googlesource.com/platform/frameworks/base/+/master/tools/aapt2/)
- [Rust Programming Language](https://www.rust-lang.org/)
