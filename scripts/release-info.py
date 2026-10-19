#!/usr/bin/env python3
"""输入：仓库文件及可选 RELEASE_TAG；输出：通过校验的 Cargo 版本。"""

import json
import os
from pathlib import Path
import plistlib
import subprocess


def main():
    """输入：环境与当前仓库；返回：无，版本不一致时以非零状态退出。"""
    root = Path(__file__).resolve().parent.parent
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=root, text=True,
    ))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "finder-jump")
    with (root / "assets/Info.plist").open("rb") as source:
        bundle = plistlib.load(source)
    base_version = version.split("-", 1)[0].split("+", 1)[0]
    if bundle["CFBundleShortVersionString"] != base_version:
        raise SystemExit(f"Info.plist 版本必须为 {base_version}，与 Cargo.toml 对齐")
    tag = os.environ.get("RELEASE_TAG", "")
    if tag and tag != f"v{version}":
        raise SystemExit(f"版本标签必须为 v{version}，实际为 {tag}")
    print(version)


if __name__ == "__main__":
    main()
