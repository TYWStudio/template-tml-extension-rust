@echo off
setlocal
if not exist bin mkdir bin
where cargo >nul 2>nul
if errorlevel 1 (
    echo 未找到 cargo，请先安装：https://rustup.rs
    exit /b 1
)
rem SDK 来自 TML 仓库的 git 依赖（首次构建会 clone 一次，之后走 cargo 缓存）
cargo build --release
if errorlevel 1 exit /b 1
copy /y target\release\tml_ext.exe bin\tml_ext.exe >nul
python build.py
echo 完成：template-tml-extension-rust.tmlext
