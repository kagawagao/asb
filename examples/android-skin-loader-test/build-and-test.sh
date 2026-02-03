#!/bin/bash

# Android Skin Loader Test - 构建和测试脚本
# 这个脚本演示如何构建皮肤包并测试资源加载

set -e

echo "=================================="
echo "Android Skin Loader Test"
echo "=================================="
echo ""

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查 ANDROID_HOME
if [ -z "$ANDROID_HOME" ]; then
    echo -e "${RED}错误: ANDROID_HOME 环境变量未设置${NC}"
    echo "请设置 ANDROID_HOME 到你的 Android SDK 路径"
    exit 1
fi

echo -e "${GREEN}✓ ANDROID_HOME: $ANDROID_HOME${NC}"

# 检查 ASB 工具
ASB_PATH="../../target/release/asb"
if [ ! -f "$ASB_PATH" ]; then
    echo -e "${RED}错误: ASB 工具未找到${NC}"
    echo "请先构建 ASB: cd ../.. && cargo build --release"
    exit 1
fi

echo -e "${GREEN}✓ ASB tool found${NC}"

# 步骤 1: 构建皮肤包
echo ""
echo "=================================="
echo "步骤 1: 构建皮肤包"
echo "=================================="

$ASB_PATH build --config asb.config.json

if [ ! -f "build/skin/com.example.skin.skin" ]; then
    echo -e "${RED}错误: 皮肤包构建失败${NC}"
    exit 1
fi

echo -e "${GREEN}✓ 皮肤包构建成功${NC}"

# 验证 Package ID
echo ""
echo "=================================="
echo "步骤 2: 验证 Package ID"
echo "=================================="

# 查找 aapt
AAPT=""
if command -v aapt &> /dev/null; then
    AAPT="aapt"
else
    # 在 Android SDK 中查找 aapt
    AAPT=$(find "$ANDROID_HOME/build-tools" -name "aapt" | sort -r | head -n 1)
fi

if [ -z "$AAPT" ]; then
    echo -e "${YELLOW}警告: aapt 未找到，跳过 Package ID 验证${NC}"
else
    echo "使用 aapt: $AAPT"
    echo ""
    echo "皮肤包资源信息:"
    $AAPT dump resources build/skin/com.example.skin.skin | head -n 20
    
    echo ""
    # 检查 Package ID
    if $AAPT dump resources build/skin/com.example.skin.skin | grep -q "id=0x7f"; then
        echo -e "${GREEN}✓ Package ID 正确设置为 0x7f${NC}"
    else
        echo -e "${YELLOW}警告: Package ID 可能不是 0x7f${NC}"
        echo "这可能导致 getIdentifier() 返回 0"
    fi
fi

# 显示皮肤包信息
echo ""
echo "皮肤包信息:"
echo "  路径: build/skin/com.example.skin.skin"
echo "  大小: $(ls -lh build/skin/com.example.skin.skin | awk '{print $5}')"
echo "  包名: com.example.skin"

# 步骤 3: Android 应用说明
echo ""
echo "=================================="
echo "步骤 3: 构建 Android 应用"
echo "=================================="
echo ""
echo "要构建和测试 Android 应用，请执行以下步骤："
echo ""
echo "1. 在 Android Studio 中打开此项目目录"
echo "   File -> Open -> 选择 android-skin-loader-test 目录"
echo ""
echo "2. 或使用命令行构建："
echo "   cd app"
echo "   ./gradlew assembleDebug"
echo ""
echo "3. 安装应用到设备："
echo "   adb install app/build/outputs/apk/debug/app-debug.apk"
echo ""
echo "4. 推送皮肤包到设备："
echo "   adb push build/skin/com.example.skin.skin /sdcard/Download/"
echo ""
echo "5. 启动应用："
echo "   adb shell am start -n com.example.skintest/.MainActivity"
echo ""

# 如果 adb 可用，尝试自动推送
if command -v adb &> /dev/null; then
    # 检查是否有设备连接
    if adb devices | grep -q "device$"; then
        echo "检测到 Android 设备"
        read -p "是否要推送皮肤包到设备? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "推送皮肤包到设备..."
            adb push build/skin/com.example.skin.skin /sdcard/Download/
            echo -e "${GREEN}✓ 皮肤包已推送到设备${NC}"
            
            # 设置文件权限
            adb shell chmod 644 /sdcard/Download/com.example.skin.skin
            echo -e "${GREEN}✓ 文件权限已设置${NC}"
        fi
    fi
fi

echo ""
echo "=================================="
echo "完成!"
echo "=================================="
echo ""
echo "测试步骤:"
echo "1. 在设备上打开 Skin Loader Test 应用"
echo "2. 点击 'Load Skin' 按钮"
echo "3. 检查日志输出，确认："
echo "   - Resource ID 不为 0"
echo "   - 颜色预览改变为皮肤颜色"
echo "   - 状态显示 'Skin loaded successfully!'"
echo ""
echo "如果遇到问题，请查看:"
echo "  - README.md 中的故障排除部分"
echo "  - 应用内的日志输出"
echo ""
