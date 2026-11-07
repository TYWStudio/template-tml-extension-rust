//! TML Extension API v1 —— Rust 语言宿主 SDK（运行时：native / rust）
//!
//! TYW Launcher 的扩展永远运行在启动器进程之外，仅通过 stdin/stdout 的
//! NDJSON（每行一个 JSON）交换白名单方法。本文件是最小实现，只依赖 Rust
//! 标准库，不引入 serde 等第三方 crate：
//!
//!   * 自动完成 initialize 握手；
//!   * 封装 log / toast / host.info / instances.list / event.subscribe /
//!     storage.get / storage.set / settings.get / settings.set；
//!   * 接收宿主广播的事件，收到 shutdown 优雅退出。
//!
//! 可调用的能力由 manifest.json 的 permissions 决定，未申请的永远不会返回数据。
//! 详见启动器「设置 → 其他 → 拓展」或拓展协议文档 extensions/README.md。

use std::io::{self, BufRead, Write};

const MAX_LINE: usize = 64 * 1024;

// ======================= 极简 JSON 工具（够用即可） =======================

/// 把字符串转义成 JSON 字面量（不含两侧引号）。
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 在 JSON 文本里定位 `"key"` 之后的值起点。
fn value_start<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\"", key);
    let mut from = 0usize;
    while let Some(pos) = json[from..].find(&pat) {
        let mut rest = &json[from + pos + pat.len()..];
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(':') {
            return Some(r.trim_start());
        }
        from = from + pos + pat.len();
    }
    None
}

/// 取顶层字符串字段（支持 \" \\ \n \t 转义）；失败返回 None。
fn get_str(json: &str, key: &str) -> Option<String> {
    let v = value_start(json, key)?;
    let mut it = v.chars();
    if it.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for c in it {
        if escaped {
            match c {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {} // 模板不处理 \uXXXX，遇到直接忽略
                other => out.push(other),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

/// 取顶层整数字段（id / apiVersion 等）。
fn get_int(json: &str, key: &str) -> Option<i64> {
    let v = value_start(json, key)?;
    let digits: String = v.chars().take_while(|c| *c == '-' || c.is_ascii_digit()).collect();
    digits.parse::<i64>().ok()
}

/// 在 `"params":{...}` 内取字符串字段。
fn get_param_str(json: &str, key: &str) -> Option<String> {
    let v = value_start(json, "params")?;
    if !v.starts_with('{') {
        return None;
    }
    get_str(v, key)
}

/// 取出 `"result"` 对应的对象原文（用于把宿主返回值转交给调用方）。
fn get_result_obj(json: &str) -> Option<String> {
    let v = value_start(json, "result")?;
    if !v.starts_with('{') {
        return None;
    }
    let bytes = v.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for (i, b) in bytes.iter().enumerate() {
        if in_str {
            if escaped {
                escaped = false;
            } else if *b == b'\\' {
                escaped = true;
            } else if *b == b'"' {
                in_str = false;
            }
            continue;
        }
        match *b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(v[..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

// ============================ 收发与协议分发 ============================

struct Ext {
    next_id: i64,
    stop: bool,
}

impl Ext {
    fn new() -> Ext {
        Ext { next_id: 2, stop: false }
    }

    /// 向宿主发一条消息（一行 JSON）。
    fn send(&self, line: &str) {
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(line.as_bytes());
        let _ = lock.write_all(b"\n");
        let _ = lock.flush();
    }

    /// 读一行（去掉行尾换行）；EOF 返回 None。
    fn read_line(&self) -> Option<String> {
        let stdin = io::stdin();
        let mut buf = String::new();
        match stdin.lock().read_line(&mut buf) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = buf.trim_end_matches(['\r', '\n']).to_string();
                if trimmed.len() > MAX_LINE {
                    Some(trimmed[..MAX_LINE].to_string())
                } else {
                    Some(trimmed)
                }
            }
            Err(_) => None,
        }
    }

    /// 同步调用宿主能力，阻塞到拿到同 id 的响应。
    /// 返回值是 `result` 对象原文；出错或超时返回 None。
    fn call(&mut self, method: &str, params: &str) -> Option<String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&format!(
            "{{\"id\":{},\"method\":\"{}\",\"params\":{}}}",
            id,
            escape(method),
            if params.is_empty() { "{}" } else { params }
        ));
        loop {
            let line = self.read_line()?;
            if line.is_empty() {
                continue;
            }
            let has_result = line.contains("\"result\"");
            let has_error = line.contains("\"error\"");
            if (has_result || has_error) && get_int(&line, "id") == Some(id) {
                if has_error {
                    return None;
                }
                return get_result_obj(&line);
            }
            self.handle(&line);
            if self.stop {
                return None;
            }
        }
    }

    /// 处理一条来自宿主的消息。返回 false 表示应当退出。
    fn handle(&mut self, line: &str) -> bool {
        // 宿主对我们请求的响应（非当前等待的 id，忽略）
        if line.contains("\"result\"") || line.contains("\"error\"") {
            return !self.stop;
        }
        let method = match get_str(line, "method") {
            Some(m) => m,
            None => return !self.stop,
        };
        let id = get_int(line, "id").unwrap_or(0);
        match method.as_str() {
            "initialize" => {
                self.send(&format!("{{\"id\":{},\"result\":{{\"ready\":true}}}}", id));
            }
            "shutdown" => {
                self.stop = true;
            }
            "event" => {
                if let Some(t) = get_param_str(line, "type") {
                    eprintln!("[event] {}", t);
                }
            }
            _ => {
                // 宿主下发的自定义命令：模板一律确认
                self.send(&format!("{{\"id\":{},\"result\":{{}}}}", id));
            }
        }
        !self.stop
    }

    // ------------------------- 白名单能力封装 -------------------------

    fn log(&mut self, level: &str, message: &str) {
        let p = format!(
            "{{\"level\":\"{}\",\"message\":\"{}\"}}",
            escape(level),
            escape(message)
        );
        self.call("log", &p);
    }

    fn toast(&mut self, message: &str) {
        let p = format!("{{\"message\":\"{}\"}}", escape(message));
        self.call("toast", &p);
    }

    fn host_info(&mut self) -> Option<String> {
        self.call("host.info", "{}")
    }

    fn list_instances(&mut self) -> Option<String> {
        self.call("instances.list", "{}")
    }

    fn subscribe(&mut self, types: &str) -> Option<String> {
        let p = format!("{{\"types\":{}}}", types);
        self.call("event.subscribe", &p)
    }

    fn storage_set(&mut self, key: &str, value: &str) {
        let p = format!(
            "{{\"key\":\"{}\",\"value\":\"{}\"}}",
            escape(key),
            escape(value)
        );
        self.call("storage.set", &p);
    }

    fn storage_get(&mut self, key: &str) -> Option<String> {
        let p = format!("{{\"key\":\"{}\"}}", escape(key));
        self.call("storage.get", &p)
    }

    fn settings_set(&mut self, key: &str, value: &str) {
        let p = format!(
            "{{\"key\":\"{}\",\"value\":\"{}\"}}",
            escape(key),
            escape(value)
        );
        self.call("settings.set", &p);
    }

    fn settings_get(&mut self, key: &str) -> Option<String> {
        let p = format!("{{\"key\":\"{}\"}}", escape(key));
        self.call("settings.get", &p)
    }
}

fn main() {
    // 宿主下发的协议配置（环境变量）
    let ext_id = std::env::var("TML_EXT_ID").unwrap_or_else(|_| "?".to_string());
    let api = std::env::var("TML_EXT_API").unwrap_or_else(|_| "?".to_string());
    eprintln!("[Rust 扩展] id={} api={}", ext_id, api);

    let mut ext = Ext::new();

    // 等待 initialize 握手
    let mut ready = false;
    loop {
        match ext.read_line() {
            None => break,
            Some(line) => {
                if line.is_empty() {
                    continue;
                }
                let is_init = get_str(&line, "method").as_deref() == Some("initialize");
                ext.handle(&line);
                if is_init {
                    ready = true;
                    break;
                }
                if ext.stop {
                    break;
                }
            }
        }
    }
    if !ready {
        // 没握手就断开（宿主没启动我们，或 stdin 被关掉）：直接退出，不执行业务逻辑
        eprintln!("[Rust 扩展] 未收到 initialize 握手，退出");
        return;
    }

    // ===================== 下面是扩展的业务逻辑示例 =====================
    ext.log("info", "Rust 扩展已启动（TML Extension API v1）");

    if let Some(info) = ext.host_info() {
        eprintln!("[host.info] {}", info);
    }
    if let Some(list) = ext.list_instances() {
        eprintln!("[instances] {}", list);
    }
    ext.subscribe("[\"launcher.started\",\"game.launched\",\"settings.changed\"]");

    ext.storage_set("demo", "hello");
    if let Some(v) = ext.storage_get("demo") {
        eprintln!("[storage.get demo] {}", v);
    }
    ext.settings_set("foo", "bar");
    if let Some(s) = ext.settings_get("foo") {
        eprintln!("[settings.get foo] {}", s);
    }

    ext.toast("Rust 扩展已就绪");

    // 保持运行，处理事件与命令直到 shutdown
    loop {
        match ext.read_line() {
            None => break,
            Some(line) => {
                if line.is_empty() {
                    continue;
                }
                if !ext.handle(&line) {
                    break;
                }
            }
        }
    }
    eprintln!("[Rust 扩展] 已退出");
}
