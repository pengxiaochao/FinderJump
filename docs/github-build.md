# GitHub 自动打包与 Release 发布

参考 grid-planner 的工作流，保留双架构 runner、固定官方 action 提交、依赖下载缓存、ZIP 上传和 14 天 Artifact 保留策略，增加版本校验及 Release 发布。

入口：`.github/workflows/build-macos.yml`，名称「构建与发布 macOS 应用」。

## 自动触发逻辑

| 事件 | 构建双架构 | 公开 Release |
| --- | --- | --- |
| push 到 main / master | 是 | 否 |
| push 到其他普通分支 | 否 | 否 |
| PR 创建、重新打开、提交更新 | 是 | 否 |
| push v0.1.0 等 v* 标签 | 是 | 全部成功后自动发布 |
| 手动运行，publish_release=false（默认） | 是 | 否 |
| 手动选择版本标签，publish_release=true | 是 | 全部成功后发布 |
| 手动选择分支，publish_release=true | 前置检查失败 | 否 |

没有路径过滤，因此只修改 README 也会运行。`v*` 只是事件过滤：v-test 等错误标签虽然触发任务，但无法通过后续版本校验。

分支 push 与标签 push 是两个事件。同时推送主分支提交与版本标签时，会有两次工作流：分支运行生成 Artifact，标签运行发布 Release。只执行本地 git tag 或只推送分支都不会发布。

标签不要求属于 main/master，发布的是标签指向的提交。该提交必须已包含工作流、脚本和 Cargo.lock。手动入口要求工作流已存在于 GitHub 默认分支；PR 默认构建 GitHub 提供的合并结果，使用的是 pull_request 事件。

## 一次构建执行什么

1. 在 macos-15（arm64）与 macos-15-intel（x86_64）上分别构建，并检查实际 uname 架构。
2. 安装 stable Rust、rustfmt 和 Clippy，检查 Cargo.lock、Xcode、Swift。
3. 检查 Cargo.toml 的版本、Info.plist 的短版本、可选版本标签是否一致。
4. 运行格式检查、Clippy、已有测试和发布脚本的离线子进程演练。
5. 调用 scripts/package-macos.sh，内部复用 scripts/bundle.sh，构建 release、图标和 .app，完成 ad-hoc 签名及严格签名检查。
6. 检查二进制架构，用 ditto 生成 ZIP，生成并验证 SHA-256，再实际解压 ZIP，核对执行权限、签名及二进制 --help。
7. 每个架构分别上传 Artifact；普通构建到此结束。
8. 满足发布条件时，Ubuntu 发布任务等待两个构建全部成功，下载本次运行的四个附件，再执行发布脚本。

任一检查、打包、上传失败都不会执行发布任务。fail-fast=false 允许另一架构继续完成，方便排查，但单架构成功不会触发完整 Release。

同一分支的新运行会取消旧运行。标签运行不会主动取消正在执行的同标签任务，避免中断上传。GitHub concurrency 同时只保留一个运行中和一个待运行任务，不保证保留每次重复请求。

只缓存依赖下载，不共享编译产物。Cargo.lock 固定依赖版本，但 Rust stable 和 runner 镜像仍会随平台更新。

## 下载应用

进入仓库 → Actions → 对应运行 → Artifacts，下载 macOS-app-arm64 或 macOS-app-x86_64。每份包含：

```text
FinderJump-0.1.0-macOS-arm64.zip
FinderJump-0.1.0-macOS-arm64.zip.sha256
```

从 Actions 下载的外层归档需要先解压，再解压内层 ZIP，得到 FinderJump.app。两份应用是分别构建的原生版本，不是 Universal Binary。Apple Silicon 选择 arm64，Intel 选择 x86_64。

Artifact 保留 14 天，也受仓库保留策略影响；Release 附件不适用该 14 天过期规则。在两个文件所在目录核对完整性：

```bash
shasum -a 256 -c FinderJump-0.1.0-macOS-arm64.zip.sha256
```

## 发布当前版本

先把项目和新增工作流推送到 GitHub，启用 Actions，并确认要发布的修改已提交。当前版本为 0.1.0：

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

若当前仓库没有 origin，需先配置你自己的 GitHub 远端。这些是操作示例，生成工作流不会替你执行 git push。

下次发布 0.2.0 时，修改 Cargo.toml 的 package.version 和 Info.plist 的 CFBundleShortVersionString，递增 CFBundleVersion，更新并提交 Cargo.lock，再为该提交创建、推送 v0.2.0。

标签必须精确等于 v + Cargo 版本。预发布示例：Cargo 版本为 0.2.0-rc.1，标签为 v0.2.0-rc.1，Info.plist 的短版本仍为 0.2.0。预发布会自动标为 Prerelease，并禁止设为 Latest；正式版本的 Latest 选择交给 GitHub 默认规则。

## 发布步骤及失败恢复

scripts/publish-release.sh 会先检查两个 ZIP 与两个 SHA-256 文件全部存在且校验通过，再确认远端标签存在。它不会自动从默认分支创建标签。

同名 Release 不存在时，先创建草稿，自动生成变更说明，并附加 .github/release-notes.md 中的安装与签名说明；同名版本已公开时明确失败，保留原附件；仍是草稿时允许重新上传同名附件。四个附件全部上传成功后才公开。

上传失败会保留草稿。可以选择 Re-run failed jobs 恢复；前一次构建 Artifact 必须仍未过期，过期后需重新运行整个标签构建。草稿重试会替换同名附件，保留其他附件；公开版本不能通过本脚本替换，应发布新版本。

也可显式手动重跑标签发布：

```bash
gh workflow run build-macos.yml --ref v0.1.0 -f publish_release=true
```

不传 publish_release=true 时，即使手动选择标签，也只生成 Artifact。

## 本地打包后发布

单机打包当前架构：

```bash
RELEASE_TAG=v0.1.0 bash scripts/package-macos.sh
```

发布要求两个架构。先在另一架构机器打包同一版本，或收集同一标签 Actions 产物，将四个文件放入同一 dist 目录。远端标签需已推送，GitHub CLI 需已登录并拥有仓库写权限。然后可将打包与发布串联：

```bash
RELEASE_TAG=v0.1.0 bash scripts/package-macos.sh && \
RELEASE_TAG=v0.1.0 GH_REPO=你的用户名/FinderJump \
  bash scripts/publish-release.sh dist
```

第二段只在打包成功后运行，并再次检查双架构附件。仓库名替换为实际值；本地可用 gh auth login 登录，不要将令牌写入项目。

## 权限与验证边界

构建任务只有 contents:read；PR 不运行发布任务。只有两个构建成功且满足标签发布条件时，Release 任务才使用 contents:write，通过 GitHub 自动提供的 GITHUB_TOKEN 发布，通常无需额外 PAT 或 Secrets。组织 / 仓库策略需允许这些权限。

Release 直接使用本次构建的附件，不进行第二次打包，也不依赖 release.published 事件串联任务。当前 CI 为 ad-hoc 签名，没有 Developer ID 证书导入、Apple 公证、staple 或应用内自动更新。发布到 GitHub 不等于通过 Gatekeeper。

验证范围见 tests/github-release-acceptance.md。离线演练使用 gh 替身，不会调用 GitHub；本机只能验证当前架构，Intel runner 与真实 GitHub API 仍需首次线上运行确认。

官方参考：[触发事件](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)、[runner 架构](https://github.com/actions/runner-images)、[gh release create](https://cli.github.com/manual/gh_release_create)。
