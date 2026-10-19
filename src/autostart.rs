use anyhow::{Context, Result, ensure};
use plist::{Dictionary, Value};
use std::{fs, path::PathBuf};

/// 输入：无；返回：当前用户登录启动 plist 路径。
pub fn path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("Cannot determine home directory")?
        .join("Library/LaunchAgents/com.finderjump.app.plist"))
}

/// 输入：无；返回：是否已安装用户登录启动文件。
pub fn enabled() -> bool {
    path().is_ok_and(|p| p.is_file())
}

/// 输入：启用状态；返回：安装或移除结果，变更在下次登录生效。
pub fn set_enabled(enabled: bool) -> Result<()> {
    let path = path()?;
    if !enabled {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe()?.canonicalize()?;
    ensure!(
        exe.to_string_lossy().contains(".app/Contents/MacOS/"),
        "Install FinderJump.app before enabling login startup"
    );
    fs::create_dir_all(path.parent().context("Missing LaunchAgents directory")?)?;
    let mut dict = Dictionary::new();
    dict.insert("Label".into(), Value::String("com.finderjump.app".into()));
    dict.insert(
        "ProgramArguments".into(),
        Value::Array(vec![Value::String(exe.to_string_lossy().into())]),
    );
    dict.insert("RunAtLoad".into(), Value::Boolean(true));
    dict.insert("KeepAlive".into(), Value::Boolean(false));
    let temp = path.with_extension("tmp");
    Value::Dictionary(dict).to_file_xml(&temp)?;
    fs::rename(temp, path)?;
    Ok(())
}
