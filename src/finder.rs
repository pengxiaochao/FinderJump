use crate::process::applescript;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

const SCRIPT: &str = r#"
tell application "Finder"
    set itemsSelected to selection
    if (count of itemsSelected) > 0 then
        return POSIX path of (item 1 of itemsSelected as alias)
    end if
    if (count of Finder windows) > 0 then
        try
            return POSIX path of (target of front Finder window as alias)
        end try
    end if
    return ""
end tell
"#;

/// 输入：无；返回：Finder 首个选中项目，否则取窗口目录，无目标时取 HOME。
pub fn current_target() -> Result<PathBuf> {
    decode_path(&applescript(SCRIPT, &[])?).and_then(|path| {
        ensure!(
            path.is_dir() || path.is_file(),
            "Finder target is not an accessible file or directory: {}",
            path.display()
        );
        Ok(path)
    })
}

/// 输入：osascript 原始输出；返回：仅删除协议换行后的路径，保留文件名空白。
pub fn decode_path(output: &str) -> Result<PathBuf> {
    let path = output.strip_suffix('\n').unwrap_or(output);
    if path.is_empty() {
        return dirs::home_dir().context("Cannot determine home directory");
    }
    Ok(PathBuf::from(path))
}
