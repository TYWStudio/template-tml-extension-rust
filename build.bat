@echo off
setlocal
if not exist bin mkdir bin
where cargo >nul 2>nul
if %errorlevel%==0 (
    cargo build --release
    if errorlevel 1 exit /b 1
    copy /y target\release\tml_ext.exe bin\tml_ext.exe >nul
) else (
    where rustc >nul 2>nul
    if errorlevel 1 (
        echo 未找到 Rust 工具链（cargo / rustc），请先安装：https://rustup.rs
        exit /b 1
    )
    rustc -O -o bin\tml_ext.exe src\main.rs
    if errorlevel 1 exit /b 1
)
python build.py
echo 完成：template-tml-extension-rust.tmlext
