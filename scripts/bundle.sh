#!/bin/bash
# 输入：可选 CODESIGN_IDENTITY 环境变量；输出：已签名的本机架构 FinderJump.app。
set -euo pipefail
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=12.0
cargo build --release --locked
app="dist/FinderJump.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/finder-jump "$app/Contents/MacOS/finder-jump"
/usr/bin/swift -module-cache-path "$PWD/target/swift-cache" scripts/icon.swift target/FinderJump.iconset
/usr/bin/iconutil -c icns target/FinderJump.iconset -o "$app/Contents/Resources/FinderJump.icns"
cp assets/Info.plist "$app/Contents/Info.plist"
cp config.example.toml "$app/Contents/Resources/config.example.toml"
/usr/bin/plutil -lint "$app/Contents/Info.plist"
/usr/bin/codesign --force --options runtime --entitlements assets/entitlements.plist \
  --sign "${CODESIGN_IDENTITY:--}" "$app"
/usr/bin/codesign --verify --deep --strict "$app"
printf 'Built: %s/%s\n' "$PWD" "$app"
