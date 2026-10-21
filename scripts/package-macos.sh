#!/bin/bash
# 输入：可选 RELEASE_TAG / CODESIGN_IDENTITY；输出：本机架构 .app、ZIP 与 SHA-256。
set -euo pipefail
cd "$(dirname "$0")/.."
version="$(python3 scripts/release-info.py)"
arch="$(uname -m)"
case "$arch" in arm64|x86_64) ;; *) echo "Unsupported architecture: $arch" >&2; exit 1 ;; esac
bash scripts/bundle.sh
app="dist/FinderJump.app"
archive="FinderJump-${version}-macOS-${arch}.zip"
# Apple lipo 要求输入文件在前，避免路径被当成 -verify_arch 的架构参数。
/usr/bin/lipo "$app/Contents/MacOS/finder-jump" -verify_arch "$arch"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$app" "dist/$archive"
(
  cd dist
  /usr/bin/shasum -a 256 "$archive" > "$archive.sha256"
  /usr/bin/shasum -a 256 -c "$archive.sha256"
)
# 验证实际下载包解压后仍保留执行权限和有效签名，不仅验证压缩前的目录。
verify_dir="$(mktemp -d "${TMPDIR:-/tmp}/finderjump-package.XXXXXX")"
trap 'rm -rf "$verify_dir"' EXIT
/usr/bin/ditto -x -k "dist/$archive" "$verify_dir"
test -x "$verify_dir/FinderJump.app/Contents/MacOS/finder-jump"
/usr/bin/codesign --verify --deep --strict "$verify_dir/FinderJump.app"
"$verify_dir/FinderJump.app/Contents/MacOS/finder-jump" --help
printf 'Packaged: %s/dist/%s\n' "$PWD" "$archive"
