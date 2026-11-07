#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""模拟 TML 宿主，跑一遍 NDJSON 会话：握手 → 白名单能力 → 事件 → 退出。

用法：python3 tests/smoke.py ./bin/tml_ext
CI 用它验证模板编译出来的可执行确实符合 TML Extension API v1 协议。
"""
import json
import subprocess
import sys
import threading
import time

BIN = sys.argv[1] if len(sys.argv) > 1 else "./bin/tml_ext"

seen = {"ready": False, "log": False, "toast": False, "instances": False, "subscribe": False}
errors = []


def reply(proc, lock, msg):
    with lock:
        proc.stdin.write(json.dumps(msg) + "\n")
        proc.stdin.flush()


def reader(proc, lock):
    for raw in proc.stdout:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            errors.append("扩展输出了非 JSON 行：" + line[:120])
            continue

        # 扩展对我们请求的响应
        if "result" in msg or "error" in msg:
            if isinstance(msg.get("result"), dict) and msg["result"].get("ready"):
                seen["ready"] = True
            if "error" in msg:
                errors.append("扩展返回错误：" + json.dumps(msg["error"], ensure_ascii=False))
            continue

        method = msg.get("method")
        mid = msg.get("id")
        result = {}
        if method == "log":
            seen["log"] = True
        elif method == "toast":
            seen["toast"] = True
        elif method == "host.info":
            result = {"platform": "linux", "apiVersion": 1, "launcherVersion": "1.1.40",
                      "channel": "Release", "language": "zh-CN", "architecture": "x64"}
        elif method == "instances.list":
            seen["instances"] = True
            result = {"instances": [{"id": "1", "name": "测试服", "gameVersion": "1.20.1", "loader": "fabric"}]}
        elif method == "event.subscribe":
            seen["subscribe"] = True
            result = {"accepted": (msg.get("params") or {}).get("types", [])}
        elif method in ("storage.get", "storage.set", "settings.get", "settings.set"):
            result = {"value": None}
        if mid is not None:
            reply(proc, lock, {"id": mid, "result": result})


def main():
    proc = subprocess.Popen(
        [BIN],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        bufsize=1,
    )
    lock = threading.Lock()
    t = threading.Thread(target=reader, args=(proc, lock), daemon=True)
    t.start()

    reply(proc, lock, {
        "id": 1,
        "method": "initialize",
        "params": {
            "apiVersion": 1,
            "extId": "ci",
            "platform": "linux",
            "launcherVersion": "1.1.40",
            "channel": "Release",
            "language": "zh-CN",
            "privateDir": "/tmp/tml-ext-ci",
            "permissions": ["log", "toast", "host.info", "instances.list",
                            "events", "storage", "settings"],
            "nonce": "n",
        },
    })
    time.sleep(1.2)
    reply(proc, lock, {"method": "event", "params": {"type": "game.launched",
                                                     "data": {"instanceId": "1"}}})
    time.sleep(0.2)
    with lock:
        proc.stdin.write(json.dumps({"method": "shutdown"}) + "\n")
        proc.stdin.flush()
    try:
        proc.wait(timeout=6)
    except Exception:
        proc.kill()
        errors.append("扩展未在收到 shutdown 后退出")
    t.join(timeout=2)

    stderr = (proc.stderr.read() if proc.stderr else "") or ""
    ok = True
    for key, label in [("ready", "initialize 握手"), ("log", "log 调用"),
                       ("instances", "instances.list 调用"), ("subscribe", "event.subscribe 调用")]:
        if not seen[key]:
            errors.append("未观察到：" + label)
            ok = False
    if errors:
        print("SMOKE FAILED")
        for e in errors:
            print(" -", e)
        print("--- stderr ---")
        print(stderr[-2000:])
        return 1
    print("SMOKE OK (exit=%s)" % proc.returncode)
    return 0


if __name__ == "__main__":
    sys.exit(main())
