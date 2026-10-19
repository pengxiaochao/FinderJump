use anyhow::{Context, Result, bail};
use std::{
    ffi::OsStr,
    process::{Command, Output, Stdio},
    thread,
};

/// 输入：命令；返回：成功的捕获输出或含 stderr 的错误。
pub fn checked(command: &mut Command) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("Run {:?}", command.get_program()))?;
    if !output.status.success() {
        bail!(
            "{:?}: {} ({})",
            command.get_program(),
            String::from_utf8_lossy(&output.stderr).trim(),
            output.status
        );
    }
    Ok(output)
}

/// 输入：固定 AppleScript 和独立参数；返回：脚本 stdout，路径不拼接进脚本源码。
pub fn applescript(script: &str, args: &[&OsStr]) -> Result<String> {
    let output = checked(
        Command::new("/usr/bin/osascript")
            .args(["-e", script, "--"])
            .args(args),
    )?;
    String::from_utf8(output.stdout).context("AppleScript returned invalid UTF-8")
}

/// 输入：长期运行的应用命令；返回：启动结果，后台回收子进程以避免僵尸进程。
pub fn detached(command: &mut Command) -> Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
