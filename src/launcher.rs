use crate::{
    config::Config,
    finder,
    process::{applescript, checked, detached},
};
use anyhow::{Context, Result, ensure};
use std::{
    env,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Terminal,
    Editor,
}

/// 输入：动作及配置；返回：优先打开 Finder 选中目标，终端对文件使用父目录的结果。
pub fn execute(action: Action, config: &Config) -> Result<()> {
    let target = finder::current_target()?;
    let path = match action {
        Action::Terminal if target.is_file() => target
            .parent()
            .context("Selected file has no parent directory")?,
        _ => target.as_path(),
    };
    match action {
        Action::Terminal if config.terminal.kind == "iterm2" => {
            checked(Command::new("/usr/bin/open").args(["-Rb", "com.googlecode.iterm2"]))?;
            applescript(
                include_str!("../assets/iterm.applescript"),
                &[path.as_os_str()],
            )?;
            Ok(())
        }
        Action::Terminal => alacritty(path, config),
        Action::Editor => vscode(path, config),
    }
}

/// 输入：路径；返回：是否为可执行常规文件。
fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// 输入：用户覆盖值、CLI 名称和应用内路径；返回：首个可执行文件或明确错误。
pub fn resolve(configured: Option<&str>, name: &str, bundle_path: &str) -> Result<PathBuf> {
    if let Some(value) = configured.filter(|s| !s.is_empty()) {
        let path = PathBuf::from(value);
        ensure!(
            path.is_absolute() && executable(&path),
            "Invalid executable: {value}"
        );
        return Ok(path);
    }
    let mut paths = vec![PathBuf::from("/Applications").join(bundle_path)];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Applications").join(bundle_path));
    }
    paths.extend(
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(|p| Path::new(p).join(name)),
    );
    if let Some(value) = env::var_os("PATH") {
        paths.extend(env::split_paths(&value).map(|p| p.join(name)));
    }
    paths.into_iter().find(|p| executable(p)).with_context(|| {
        format!("Cannot find {name}; set an absolute executable path in Preferences")
    })
}

/// 输入：目标目录和配置；返回：IPC、新进程或显式原生 Tab 模式的启动结果。
fn alacritty(path: &Path, config: &Config) -> Result<()> {
    let exe = resolve(
        config.terminal.executable.as_deref(),
        "alacritty",
        "Alacritty.app/Contents/MacOS/alacritty",
    )?;
    if config.terminal.alacritty_mode == "native_tab" {
        let running = applescript("return application \"Alacritty\" is running", &[])?;
        if running.trim() == "true" {
            applescript(
                include_str!("../assets/alacritty-tab.applescript"),
                &[path.as_os_str()],
            )?;
            return Ok(());
        }
    } else if checked(
        Command::new(&exe)
            .args(["msg", "create-window", "--working-directory"])
            .arg(path),
    )
    .is_ok()
    {
        checked(Command::new("/usr/bin/open").args(["-a", "Alacritty"]))?;
        return Ok(());
    }
    detached(Command::new(exe).arg("--working-directory").arg(path))
}

/// 输入：目标文件或目录和配置；返回：VS Code CLI 执行结果，保留复用或新建窗口语义。
fn vscode(path: &Path, config: &Config) -> Result<()> {
    let exe = resolve(
        config.editor.executable.as_deref(),
        "code",
        "Visual Studio Code.app/Contents/Resources/app/bin/code",
    )?;
    let flag = if config.editor.reuse_window {
        "--reuse-window"
    } else {
        "--new-window"
    };
    checked(Command::new(exe).arg(flag).arg(path))?;
    Ok(())
}
