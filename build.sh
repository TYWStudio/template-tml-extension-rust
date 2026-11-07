#!/usr/bin/env bash
# 构建 Rust 扩展并打包为 .tmlext
set -e
mkdir -p bin

if command -v cargo >/dev/null 2>&1; then
    cargo build --release
    cp target/release/tml_ext bin/tml_ext
elif command -v rustc >/dev/null 2>&1; then
    rustc -O -o bin/tml_ext src/main.rs
else
    echo "未找到 Rust 工具链（cargo / rustc），请先安装：https://rustup.rs" >&2
    exit 1
fi

chmod +x bin/tml_ext
python3 build.py
echo "完成：template-tml-extension-rust.tmlext"
