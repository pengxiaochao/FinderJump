use anyhow::{Context, Result, ensure};
use global_hotkey::hotkey::HotKey;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hotkeys: Hotkeys,
    pub terminal: Terminal,
    pub editor: Editor,
    pub logging: Logging,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Hotkeys {
    pub terminal: String,
    pub editor: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Terminal {
    pub kind: String,
    pub executable: Option<String>,
    pub alacritty_mode: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Editor {
    pub kind: String,
    pub executable: Option<String>,
    pub reuse_window: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Logging {
    pub enabled: bool,
}

impl Default for Hotkeys {
    /// 输入：无；返回：默认的终端和编辑器快捷键。
    fn default() -> Self {
        Self {
            terminal: "cmd+g".into(),
            editor: "cmd+e".into(),
        }
    }
}
impl Default for Terminal {
    /// 输入：无；返回：默认 iTerm2 终端配置。
    fn default() -> Self {
        Self {
            kind: "iterm2".into(),
            executable: None,
            alacritty_mode: "ipc".into(),
        }
    }
}
impl Default for Editor {
    /// 输入：无；返回：复用窗口的 VS Code 配置。
    fn default() -> Self {
        Self {
            kind: "vscode".into(),
            executable: None,
            reuse_window: true,
        }
    }
}
impl Default for Config {
    /// 输入：无；返回：全部默认配置。
    fn default() -> Self {
        Self {
            hotkeys: Hotkeys::default(),
            terminal: Terminal::default(),
            editor: Editor::default(),
            logging: Logging::default(),
        }
    }
}

impl Config {
    /// 输入：无；返回：用户 Application Support 下的配置文件路径。
    pub fn path() -> Result<PathBuf> {
        Ok(dirs::config_dir()
            .context("Cannot locate Application Support")?
            .join("FinderJump/config.toml"))
    }

    /// 输入：配置文件路径；返回：已验证配置；不存在时原子写入默认值。
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            return Ok(config);
        }
        let config: Self = toml::from_str(
            &fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?,
        )?;
        config.validate()?;
        Ok(config)
    }

    /// 输入：配置路径；返回：保存结果；先写临时文件再替换，避免截断旧文件。
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path.parent().context("Config path has no parent")?;
        fs::create_dir_all(parent)?;
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temp, toml::to_string_pretty(self)?)?;
        fs::rename(&temp, path).context("Replace config atomically")
    }

    /// 输入：当前配置；返回：两个已解析快捷键；拒绝重复、无修饰键和未知选项。
    pub fn validate(&self) -> Result<[HotKey; 2]> {
        ensure!(
            ["iterm2", "alacritty"].contains(&self.terminal.kind.as_str()),
            "Unsupported terminal"
        );
        ensure!(
            ["ipc", "native_tab"].contains(&self.terminal.alacritty_mode.as_str()),
            "Unsupported Alacritty mode"
        );
        ensure!(self.editor.kind == "vscode", "Unsupported editor");
        let terminal: HotKey = self
            .hotkeys
            .terminal
            .parse()
            .context("Invalid terminal shortcut")?;
        let editor: HotKey = self
            .hotkeys
            .editor
            .parse()
            .context("Invalid editor shortcut")?;
        ensure!(terminal.id() != editor.id(), "Shortcuts must be different");
        ensure!(
            [terminal, editor].iter().all(|key| key.mods.intersects(
                global_hotkey::hotkey::Modifiers::META
                    | global_hotkey::hotkey::Modifiers::CONTROL
                    | global_hotkey::hotkey::Modifiers::ALT
            )),
            "Shortcuts need Command, Control or Option"
        );
        Ok([terminal, editor])
    }
}
