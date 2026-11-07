<p align="right">
  <b>English</b> · <a href="README-CN.md">简体中文</a>
</p>

# TML Extension Template · Rust

A **Rust** extension template for the **TYW Launcher (TML)** extension API v1. It shows how to write a
launcher extension in plain Rust — **standard library only, no crates** — where the host (launcher)
spawns your compiled native binary in a separate process and the two sides exchange allow-listed
methods over **NDJSON on stdin/stdout** (one JSON object per line).

> License: **CC BY-NC 4.0** (non-commercial, attribution). See `LICENSE`.
>
> Part of the **TML** ecosystem · launcher repository: <https://github.com/TYWStudio/TML>

---

## How it runs

1. You import the packaged `*.tmlext` (essentially a zip) via **Settings → Other → Extensions** in the launcher.
2. The launcher unpacks it and starts the native binary inside the package (`entry` in `manifest.json`).
3. The launcher sends an `initialize` handshake first; the extension replies `{"ready":true}`.
4. After that the extension calls host capabilities with requests carrying `id≥2` (`log` / `host.info` /
   `storage` / `settings` …); the host broadcasts events as `event` lines and asks the extension to
   exit gracefully with `shutdown`.

Extensions are **disabled by default**, must be enabled manually by the user, and may only call the
capabilities declared in the manifest's `permissions` — calls that were not requested return no data.

## Protocol (minimal)

```
Host → Ext: {"id":1,"method":"initialize","params":{...}}    Ext → Host: {"id":1,"result":{"ready":true}}
Ext → Host: {"id":2,"method":"log","params":{"level":"info","message":"hi"}}  Host → Ext: {"id":2,"result":{}}
Host → Ext: {"method":"event","params":{"type":"game.launched","data":{...}}}
Host → Ext: {"method":"shutdown"}                                (the extension just exits)
```

`src/main.rs` contains a dependency-free JSON reader/writer and the request/response plumbing, ready
to use. The host also passes configuration through environment variables:

| Variable | Meaning |
| --- | --- |
| `TML_EXT_ID` | Unique extension id |
| `TML_EXT_API` | API version (currently 1) |
| `TML_EXT_PLATFORM` | Platform (windows / linux / macos) |
| `TML_EXT_LAUNCHER_VERSION` | Launcher version |
| `TML_EXT_CHANNEL` | Channel (Release / Snapshot / Beta) |
| `TML_EXT_LANGUAGE` | UI language |
| `TML_EXT_PRIVATE` | Extension-private data directory |
| `TML_EXT_NONCE` | One-time nonce |
| `TML_EXT_PERMISSIONS` | Granted capabilities (comma-separated) |

---

## Build

Requires a Rust toolchain: **Cargo** (preferred) or a bare `rustc`. No crates are downloaded, so the
build works offline. Python 3 is used for packaging.

### Linux / macOS

```bash
./build.sh
# output: template-tml-extension-rust.tmlext
```

### Windows

```bat
build.bat
 rem output: template-tml-extension-rust.tmlext
```

`build.sh` / `build.bat` run `cargo build --release` (falling back to `rustc -O src/main.rs`), copy the
binary to `bin/tml_ext[.exe]`, then `build.py` packs `manifest.json` + sources + the binary into the
`.tmlext`.

## Test

`tests/smoke.py` plays the host role: it spawns the built binary, performs the handshake, answers the
allow-listed calls, broadcasts an event and sends `shutdown`. CI runs it on every push.

```bash
python3 tests/check_manifest.py rust   # manifest sanity
python3 tests/smoke.py ./bin/tml_ext   # protocol smoke test
```

## Install into the launcher

Copy the generated `template-tml-extension-rust.tmlext` into **Settings → Other → Extensions** and
click **Import**. Then review the permission list, enable it, and watch the extension's debug output
in the launcher console.

## manifest.json key fields

| Field | Description |
| --- | --- |
| `runtime` | `rust` — the host treats it as a native executable (same path as `c` / `native`) |
| `format` | Must be `tmlext` (matches the file extension) |
| `entry` | Package-relative path to the binary, e.g. `bin/tml_ext` (`.exe` is appended automatically on Windows) |
| `platforms` | The current platform must be included (`windows` / `linux` / `macos`) |
| `permissions` | Requested capability allow-list |
| `subscribe` | Event types to receive |

## Layout

```
template-tml-extension-rust/
├── manifest.json         # Extension manifest
├── Cargo.toml            # No dependencies; produces bin target `tml_ext`
├── src/main.rs           # std-only NDJSON SDK + example logic
├── build.py              # Cross-platform packaging into .tmlext
├── build.sh / build.bat  # Compile + package
├── tests/                # Manifest check + host-side protocol smoke test
├── README.md
└── LICENSE               # CC BY-NC 4.0
```
