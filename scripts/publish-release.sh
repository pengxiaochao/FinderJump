#!/bin/bash
# 输入：附件目录（默认 dist）、RELEASE_TAG、GH_REPO 与 gh 登录凭据；输出：完整的 GitHub Release。
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
assets_dir="${1:-$root/dist}"
: "${RELEASE_TAG:?Set RELEASE_TAG, for example v0.1.0}"
: "${GH_REPO:?Set GH_REPO to owner/repository}"
if [[ ! "$RELEASE_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid version tag: $RELEASE_TAG" >&2
  exit 1
fi
version="${RELEASE_TAG#v}"
assets_dir="$(cd "$assets_dir" && pwd)"
files=()
for arch in arm64 x86_64; do
  archive="FinderJump-${version}-macOS-${arch}.zip"
  test -s "$assets_dir/$archive"
  test -s "$assets_dir/$archive.sha256"
  (cd "$assets_dir" && shasum -a 256 -c "$archive.sha256")
  files+=("$assets_dir/$archive" "$assets_dir/$archive.sha256")
done
prerelease=false
[[ "${version%%+*}" != *-* ]] || prerelease=true
# 即使恢复草稿，也确认远端标签仍存在，避免发布时从默认分支补建标签。
gh api "repos/$GH_REPO/git/ref/tags/$RELEASE_TAG" --silent
if draft="$(gh release view "$RELEASE_TAG" --repo "$GH_REPO" --json isDraft --jq .isDraft)"; then
  if [[ "$draft" != true ]]; then
    echo "Release already published; refusing to replace assets: $RELEASE_TAG" >&2
    exit 1
  fi
else
  gh release create "$RELEASE_TAG" --repo "$GH_REPO" --verify-tag --draft \
    --title "FinderJump $RELEASE_TAG" --prerelease="$prerelease" \
    --generate-notes --notes-file "$root/.github/release-notes.md"
fi
# 仅对草稿重试上传；全部成功后才公开，已发布版本的附件保持不变。
gh release upload "$RELEASE_TAG" "${files[@]}" --repo "$GH_REPO" --clobber
publish_flags=(--draft=false --prerelease="$prerelease")
[[ "$prerelease" == false ]] || publish_flags+=(--latest=false)
gh release edit "$RELEASE_TAG" --repo "$GH_REPO" "${publish_flags[@]}"
