<p align="right">
  <a href="README.md">English</a> · <b>简体中文</b>
</p>

> 本仓库属于 **TML** 扩展生态；启动器仓库：<https://github.com/TYWStudio/TML>

# TML 扩展模板 · Rust（template-tml-extension-rust）

适用于 **TYW Launcher（TML）扩展 API v1** 的 **Rust 语言**扩展模板。本模板演示如何用纯 Rust
（**只用标准库，不引入任何 crate**）编写启动器扩展：宿主（启动器）在独立进程里拉起你编译好的
原生可执行文件，双方仅通过 **stdin / stdout 的 NDJSON（每行一个 JSON）** 交换白名单方法。

> 许可：**CC BY-NC 4.0**（禁止商业使用，署名）。见 `LICENSE`。

---

## 它是怎么跑起来的

1. 你在启动器「设置 → 其他 → 拓展」里导入打包好的 `*.tmlext`（本质是一个 zip）。
2. 启动器解压并启动包里的原生可执行文件（`manifest.json` 的 `entry`）。
3. 启动器先发一条 `initialize` 握手，扩展回 `{"ready":true}`。
4. 之后扩展通过 `id≥2` 的请求调用宿主能力（`log` / `host.info` / `storage` / `settings` …），
   宿主通过 `event` 把事件广播给扩展，并通过 `shutdown` 通知扩展优雅退出。

扩展**默认全部禁用**、需用户手动启用，且只能调用 `manifest.json` 的 `permissions` 中申请过的
能力——未申请的调用宿主不会返回数据。

## 协议（极简）

```
宿主 → 扩展: {"id":1,"method":"initialize","params":{...}}    扩展 → 宿主: {"id":1,"result":{"ready":true}}
扩展 → 宿主: {"id":2,"method":"log","params":{"level":"info","message":"hi"}}  宿主 → 扩展: {"id":2,"result":{}}
宿主 → 扩展: {"method":"event","params":{"type":"game.launched","data":{...}}}
宿主 → 扩展: {"method":"shutdown"}                                （扩展直接退出）
```

`src/main.rs` 内含零依赖的 JSON 读写与请求/响应骨架，开箱即用；宿主还会通过环境变量下发配置：

| 变量 | 含义 |
| --- | --- |
| `TML_EXT_ID` | 扩展唯一 id |
| `TML_EXT_API` | API 版本（当前为 1） |
| `TML_EXT_PLATFORM` | 平台（windows / linux / macos） |
| `TML_EXT_LAUNCHER_VERSION` | 启动器版本 |
| `TML_EXT_CHANNEL` | 通道（Release / Snapshot / Beta） |
| `TML_EXT_LANGUAGE` | 界面语言 |
| `TML_EXT_PRIVATE` | 扩展私有数据目录 |
| `TML_EXT_NONCE` | 一次性随机数 |
| `TML_EXT_PERMISSIONS` | 已授予的能力（逗号分隔） |

---

## 构建

需要 Rust 工具链：**Cargo**（推荐）或裸 `rustc`。模板不依赖任何 crate，离线也能构建；
打包用 Python 3。

### Linux / macOS

```bash
./build.sh
# 产物：template-tml-extension-rust.tmlext
```

### Windows

```bat
build.bat
 rem 产物：template-tml-extension-rust.tmlext
```

脚本会先 `cargo build --release`（没有 Cargo 时回退 `rustc -O src/main.rs`），把二进制复制到
`bin/tml_ext[.exe]`，再由 `build.py` 将 `manifest.json` + 源码 + 二进制打成 `.tmlext`。

## 测试

`tests/smoke.py` 扮演宿主：拉起编译好的二进制、完成握手、应答白名单调用、广播事件并发送
`shutdown`。CI 每次推送都会跑一遍。

```bash
python3 tests/check_manifest.py rust   # 清单体检
python3 tests/smoke.py ./bin/tml_ext   # 协议冒烟测试
```

## 装进启动器

把生成的 `template-tml-extension-rust.tmlext` 拿到启动器「设置 → 其他 → 拓展」点 **导入**，
再查看权限列表并启用，即可在启动器控制台看到本模板的调试输出。

## manifest.json 关键字段

| 字段 | 说明 |
| --- | --- |
| `runtime` | 填 `rust`，宿主按原生可执行处理（与 `c` / `native` 同一条路径） |
| `format` | 必须为 `tmlext`（与后缀一致） |
| `entry` | 包内二进制路径，如 `bin/tml_ext`（Windows 下自动补 `.exe`） |
| `platforms` | 必须包含当前平台（`windows` / `linux` / `macos`） |
| `permissions` | 申请的能力白名单 |
| `subscribe` | 想接收的事件类型 |

## 目录结构

```
template-tml-extension-rust/
├── manifest.json         # 扩展清单
├── Cargo.toml            # 无依赖，产出名为 tml_ext 的二进制
├── src/main.rs           # 仅标准库的 NDJSON SDK + 示例逻辑
├── build.py              # 跨平台打包为 .tmlext
├── build.sh / build.bat  # 编译 + 打包
├── tests/                # 清单校验 + 宿主侧协议冒烟测试
├── README.md / README-CN.md
└── LICENSE               # CC BY-NC 4.0
```
