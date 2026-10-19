#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod app;
mod autostart;
mod config;
mod finder;
mod hotkeys;
mod launcher;
mod logging;
mod menu;
mod process;
mod settings;

use anyhow::{Context, Result, bail};
use std::{
    fs::{self, File, OpenOptions},
    os::fd::AsRawFd,
};

/// 输入：无；返回：无，启动失败在标准错误和原生对话框中显示。
fn main() {
    if let Err(error) = run() {
        eprintln!("FinderJump: {error:#}");
        if std::env::args().len() == 1 {
            let message =
                format!("{error:#}\n\nCheck ~/Library/Application Support/FinderJump/config.toml");
            let script = r#"on run argv
                display dialog (item 1 of argv) with title "FinderJump" buttons {"OK"} default button "OK" with icon caution
            end run"#;
            let _ = process::applescript(script, &[std::ffi::OsStr::new(&message)]);
        }
        std::process::exit(1);
    }
}

/// 输入：无；返回：程序退出状态；所有 Cocoa 和快捷键对象均在主线程创建。
fn run() -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("FinderJump requires macOS");
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help") {
        println!(
            "FinderJump [--preferences | --check-config | --print-config-path | --terminal | --editor]"
        );
        return Ok(());
    }
    let path = config::Config::path()?;
    if args.first().is_some_and(|a| a == "--print-config-path") {
        println!("{}", path.display());
        return Ok(());
    }
    let config = config::Config::load(&path)?;
    match args.first().map(String::as_str) {
        Some("--check-config") => {
            config.validate()?;
            println!("Configuration OK");
            return Ok(());
        }
        Some("--terminal") => return launcher::execute(launcher::Action::Terminal, &config),
        Some("--editor") => return launcher::execute(launcher::Action::Editor, &config),
        None | Some("--preferences") => {}
        Some(other) => bail!("Unknown argument: {other}"),
    }
    let _lock = lock_instance()?;
    let visible = args.contains(&"--preferences".into());
    app::run(config, path, visible)
}

/// 输入：无；返回：持有排他锁的文件，退出自动释放，阻止重复注册全局快捷键。
fn lock_instance() -> Result<File> {
    let dir = config::Config::path()?
        .parent()
        .context("Missing config directory")?
        .to_owned();
    fs::create_dir_all(&dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("instance.lock"))?;
    // SAFETY：文件描述符在调用及锁持有期间有效，flock 不涉及用户内存指针。
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        bail!("FinderJump is already running; use its menu bar icon");
    }
    Ok(file)
}

#[cfg(test)]
mod tests;
