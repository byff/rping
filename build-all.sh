#!/bin/bash
# RPing 交叉编译脚本
# 使用前请确保安装了以下依赖：
#
# Ubuntu/Debian:
#   sudo apt install gcc-mingw-w64-x86-64 gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
#
# Rust targets:
#   rustup target add x86_64-pc-windows-gnu aarch64-unknown-linux-gnu
#

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_DIR"

OUTPUT_DIR="$PROJECT_DIR/dist"
mkdir -p "$OUTPUT_DIR"

echo "========================================="
echo "  RPing 交叉编译"
echo "========================================="

# 1. x86_64 Linux (本机)
echo ""
echo "[1/3] 编译 x86_64 Linux..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/rping "$OUTPUT_DIR/rping-linux-x86_64"
echo "  -> dist/rping-linux-x86_64"

# 2. x86_64 Windows
echo ""
echo "[2/3] 编译 x86_64 Windows..."
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/rping.exe "$OUTPUT_DIR/rping-windows-x86_64.exe"
echo "  -> dist/rping-windows-x86_64.exe"

# 3. aarch64 Linux (麒麟系统)
echo ""
echo "[3/3] 编译 aarch64 Linux (麒麟/Kylin)..."
cargo build --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/rping "$OUTPUT_DIR/rping-linux-aarch64"
echo "  -> dist/rping-linux-aarch64"

echo ""
echo "========================================="
echo "  编译完成！输出目录: dist/"
echo "========================================="
ls -lh "$OUTPUT_DIR"/rping-*

# 可选：UPX 压缩（进一步减小体积）
if command -v upx &> /dev/null; then
    echo ""
    echo "检测到 UPX，正在压缩..."
    upx --best "$OUTPUT_DIR/rping-linux-x86_64" || true
    upx --best "$OUTPUT_DIR/rping-windows-x86_64.exe" || true
    upx --best "$OUTPUT_DIR/rping-linux-aarch64" || true
    echo ""
    echo "压缩后:"
    ls -lh "$OUTPUT_DIR"/rping-*
fi
