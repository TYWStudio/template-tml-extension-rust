#!/usr/bin/env bash
# 构建 Rust 扩展并打包为 .tmlext
set -e
mkdir -p bin

if ! command -v cargo >/dev/null 2>&1; then
    echo "未找到 cargo，请先安装：https://rustup.rs" >&2
    exit 1
fi

# SDK 来自 TML 仓库的 git 依赖（首次构建会 clone 一次，之后走 cargo 缓存）
cargo build --release
cp target/release/tml_ext bin/tml_ext

chmod +x bin/tml_ext
python3 build.py
echo "完成：template-tml-extension-rust.tmlext"
