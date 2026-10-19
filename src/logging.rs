use anyhow::Result;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

/// 输入：配置文件路径；返回：已创建的日志文件路径，不改变日志开关。
pub fn prepare(config_path: &Path) -> Result<PathBuf> {
    let path = config_path.with_file_name("finderjump.log");
    OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(path)
}

/// 输入：配置路径、日志开关及消息；返回：写入结果，关闭时不访问文件。
pub fn write(config_path: &Path, enabled: bool, message: &str) -> Result<()> {
    if !enabled {
        return Ok(());
    }
    let path = prepare(config_path)?;
    let mut file = OpenOptions::new().append(true).open(path)?;
    writeln!(file, "{:?} {message}", std::time::SystemTime::now())?;
    Ok(())
}
