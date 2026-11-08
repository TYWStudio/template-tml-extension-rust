//! TML 扩展模板 · Rust
//!
//! 业务代码只写在这个文件里。NDJSON 会话、initialize 握手、按 id 分发请求、
//! 白名单能力（log / toast / host.info / instances.list / storage / settings）
//! 全部来自 TML 仓库的 extensions/sdk/rust（crate `tml-ext`）—— 通过 Cargo.toml
//! 的 git 依赖引用，不要复制 lib.rs 进来。
//!
//! 宿主（TYW Launcher）把 .tmlext 解开后，直接把包里 bin/tml_ext 当独立进程拉起，
//! 双方只用 stdin/stdout 通信；能调用什么由 manifest.json 的 permissions 决定。

use tml_ext::{Event, Ext, Handler};

/// 业务入口，两个方法都可以留空。
struct Extension;

impl Handler for Extension {
    /// 握手成功后调用一次，适合做初始化。
    fn ready(&mut self, ext: &mut Ext) {
        ext.log("info", "扩展已启动（TML Extension API v1）");

        // 宿主信息：平台、架构、启动器版本、通道、语言
        if let Some(info) = ext.host_info() {
            ext.log("info", &format!("host.info = {}", info));
        }

        // 只读实例列表（需要 manifest 里申请 instances.list）
        if let Some(list) = ext.instances() {
            ext.log("info", &format!("instances = {}", list));
        }

        // 订阅事件（manifest 的 subscribe 里也要写）
        ext.subscribe(&["launcher.started", "game.launched", "settings.changed"]);

        // 私有存储：跟着扩展自己的目录走，卸载就没了
        let version = tml_ext::env("launcher_version").unwrap_or_default();
        ext.storage_set("lastRun", &version);
        if let Some(v) = ext.storage_get("lastRun") {
            ext.log("info", &format!("上次运行版本：{}", v));
        }

        // 扩展设置：显示在启动器的扩展详情里
        ext.settings_set("greeted", "1");
        if let Some(v) = ext.settings_get("greeted") {
            ext.log("info", &format!("greeted = {}", v));
        }

        ext.toast("扩展已就绪");
    }

    /// 接收订阅到的事件（数据已由宿主脱敏）。
    fn event(&mut self, ext: &mut Ext, ev: Event) {
        match ev.kind.as_str() {
            "launcher.started" => ext.log("info", "启动器已就绪"),
            "game.launched" => ext.log("info", &format!("开始游戏：{}", ev.data)),
            other => ext.log("info", &format!("事件 {}", other)),
        }
    }
}

fn main() {
    tml_ext::run(Extension);
}
