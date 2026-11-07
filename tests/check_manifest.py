#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""校验 manifest.json 是否符合 TML Extension API v1 的约定。

用法：python3 tests/check_manifest.py rust
"""
import json
import sys

EXPECTED_RUNTIME = sys.argv[1] if len(sys.argv) > 1 else ""
ALLOWED_PERMISSIONS = {"log", "toast", "host.info", "instances.list", "events", "storage", "settings"}
ALLOWED_EVENTS = {"launcher.started", "game.launched", "game.exited", "settings.changed", "instance.changed"}
ALLOWED_PLATFORMS = {"windows", "linux", "macos", "android"}


def main():
    with open("manifest.json", encoding="utf-8") as f:
        d = json.load(f)
    problems = []

    if d.get("apiVersion") != 1:
        problems.append("apiVersion 必须为 1")
    if d.get("format") != "tmlext":
        problems.append("format 必须为 tmlext")
    if EXPECTED_RUNTIME and d.get("runtime") != EXPECTED_RUNTIME:
        problems.append("runtime 必须为 %s（实际 %r）" % (EXPECTED_RUNTIME, d.get("runtime")))
    if str(d.get("id", "")).count(".") < 2:
        problems.append("id 必须使用反向域名（形如 a.b.c）")
    if not d.get("entry"):
        problems.append("缺少 entry")
    platforms = set(d.get("platforms") or [])
    if not platforms:
        problems.append("platforms 不能为空")
    if platforms - ALLOWED_PLATFORMS:
        problems.append("platforms 含未知平台：%s" % sorted(platforms - ALLOWED_PLATFORMS))
    perms = set(d.get("permissions") or [])
    if perms - ALLOWED_PERMISSIONS:
        problems.append("permissions 含未开放能力：%s" % sorted(perms - ALLOWED_PERMISSIONS))
    subs = set(d.get("subscribe") or [])
    if subs - ALLOWED_EVENTS:
        problems.append("subscribe 含未开放事件：%s" % sorted(subs - ALLOWED_EVENTS))
    if subs and "events" not in perms:
        problems.append("订阅事件必须先申请 events 权限")

    if problems:
        print("MANIFEST INVALID")
        for p in problems:
            print(" -", p)
        return 1
    print("manifest OK (runtime=%s, platforms=%s)" % (d.get("runtime"), ",".join(sorted(platforms))))
    return 0


if __name__ == "__main__":
    sys.exit(main())
