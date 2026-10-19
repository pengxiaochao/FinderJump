# GitHub 自动打包 / 发布：实现前失败场景

- 主分支 / PR / 手动构建意外发布；版本标签只打包未发布；错误标签与 Cargo / plist 版本不一致。
- Intel / Apple Silicon runner 架构错误，或复用了另一架构编译产物；锁文件缺失被悄悄更新。
- 检查或任一架构失败后仍发布；取消旧标签任务中断附件上传；重跑覆盖已公开版本。
- ZIP 丢失 .app 目录层级、执行权限或签名；校验文件不可复核；上传空产物。
- 只存在单架构包、缺少 SHA-256 或包损坏，发布脚本仍调用 GitHub。
- 标签未推送时 gh 自动创建错误标签；同名正式 Release 被覆盖；断网或上传失败留下公开半成品。
- 草稿重跑无法恢复；预发布错误设为 Latest；PR 获得写入权限。
- 发布成功被错误地描述成 Developer ID 签名 / Apple 公证 / 自动更新。

验证顺序：离线脚本流程演练（用 gh 命令替身记录边界调用，禁止真实发布）→ actionlint / ShellCheck → 本机真实构建、压缩、解压、签名及 CLI 验证。GitHub 双架构 runner、GitHub API 与真实 Release 仍需仓库首次运行确认。

## 本次执行结果（2026-10-19）

- 通过：actionlint 1.7.12（含 ShellCheck）、ShellCheck 0.11.0、Bash 语法和 git diff --check。
- 通过：11 项真实脚本子进程演练，覆盖缺少架构 / 校验文件、损坏 ZIP、非法标签、版本不匹配、新建正式版、预发布、草稿恢复、拒绝覆盖公开版、上传失败不公开、远端标签不存在。gh 使用命令替身，未访问 GitHub。
- 通过：RELEASE_TAG=v0.1.0 本机 arm64 完整打包，ZIP / SHA-256 校验、实际解压后的执行权限、严格签名验证和 --help。
- 已核对官方 macOS runner 架构与 action 固定提交；当前仓库无 Git remote，因此未推送、未运行真实 GitHub 工作流、未发布真实 Release。
- Intel 原生构建、真实 GitHub Token 权限 / Release API 和首次远端运行待确认。本机成功不能替代这些验证。
