#!/usr/bin/env python3
"""输入：本地发布脚本；输出：离线子进程演练结果，gh 替身不会访问 GitHub。"""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
FAKE_GH = '''#!/usr/bin/env python3
import json, os, sys
with open(os.environ["CALL_LOG"], "a") as output:
    output.write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[1] == "api":
    sys.exit(1 if os.environ["GH_MODE"] == "missing-tag" else 0)
action = sys.argv[2]
mode = os.environ["GH_MODE"]
if action == "view":
    if mode == "published":
        print("false")
    elif mode == "draft":
        print("true")
    else:
        sys.exit(1)
if action == "upload" and mode == "upload-fails":
    sys.exit(1)
'''


def prepare(directory, version):
    """输入：临时目录和版本；返回：带双架构附件、校验文件及 gh 替身的目录。"""
    directory = Path(directory)
    for arch in ("arm64", "x86_64"):
        archive = directory / f"FinderJump-{version}-macOS-{arch}.zip"
        archive.write_bytes(b"offline release fixture")
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_suffix(".zip.sha256").write_text(f"{digest}  {archive.name}\n")
    stub = directory / "gh"
    stub.write_text(FAKE_GH)
    stub.chmod(0o755)
    return directory


def run_case(mode="new", version="0.1.0", damage=None, tag=None):
    """输入：远端模拟状态、版本和故障；返回：脚本退出码与 gh 边界调用。"""
    with tempfile.TemporaryDirectory(prefix="finderjump-release-") as temporary:
        directory = prepare(temporary, version)
        archive = directory / f"FinderJump-{version}-macOS-x86_64.zip"
        if damage == "missing-arch":
            archive.unlink()
        elif damage == "missing-checksum":
            archive.with_suffix(".zip.sha256").unlink()
        elif damage == "corrupt":
            archive.write_bytes(b"corrupted after checksum")
        log = directory / "calls.jsonl"
        env = dict(os.environ, PATH=f"{directory}{os.pathsep}{os.environ['PATH']}",
                   RELEASE_TAG=tag or f"v{version}", GH_REPO="example/FinderJump",
                   CALL_LOG=str(log), GH_MODE=mode)
        result = subprocess.run(["bash", str(ROOT / "scripts/publish-release.sh"), str(directory)],
                                env=env, capture_output=True, text=True)
        calls = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
        return result.returncode, calls


def check_publish_paths():
    """输入：无；返回：无，验证草稿、预发布和失败重试的可见性边界。"""
    code, calls = run_case()
    assert code == 0 and calls[0][0] == "api"
    assert [c[1] for c in calls[1:]] == ["view", "create", "upload", "edit"]
    assert "--verify-tag" in calls[2] and "--draft" in calls[2]
    assert len([arg for arg in calls[3] if arg.endswith((".zip", ".sha256"))]) == 4
    assert "--draft=false" in calls[4] and "--prerelease=false" in calls[4]
    code, calls = run_case(version="0.2.0-rc.1")
    assert code == 0 and "--prerelease=true" in calls[2] and "--latest=false" in calls[-1]
    code, calls = run_case(mode="draft")
    assert code == 0 and [c[1] for c in calls[1:]] == ["view", "upload", "edit"]
    code, calls = run_case(mode="published")
    assert code != 0 and [c[1] for c in calls[1:]] == ["view"]
    code, calls = run_case(mode="upload-fails")
    assert code != 0 and [c[1] for c in calls[1:]] == ["view", "create", "upload"]
    code, calls = run_case(mode="missing-tag")
    assert code != 0 and len(calls) == 1 and calls[0][0] == "api"


def main():
    """输入：无；返回：无，发现错误时失败退出，全部通过后输出演练范围。"""
    for damage in ("missing-arch", "missing-checksum", "corrupt"):
        code, calls = run_case(damage=damage)
        assert code != 0 and not calls, damage
    code, calls = run_case(tag="not-a-version")
    assert code != 0 and not calls
    result = subprocess.run(["python3", str(ROOT / "scripts/release-info.py")],
                            env=dict(os.environ, RELEASE_TAG="v999.0.0"), capture_output=True)
    assert result.returncode != 0
    check_publish_paths()
    print("11 release subprocess scenarios passed (gh stub; no GitHub publication)")


if __name__ == "__main__":
    main()
