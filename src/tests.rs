use crate::{config::Config, finder, launcher};
use std::{fs, path::PathBuf};

/// 输入：无；返回：无，验证默认配置序列化后保持语义一致。
#[test]
fn config_round_trip() {
    let original = Config::default();
    let encoded = toml::to_string(&original).unwrap();
    let decoded: Config = toml::from_str(&encoded).unwrap();
    assert_eq!(original, decoded);
    assert!(decoded.validate().is_ok());
}

/// 输入：无；返回：无，验证等价修饰键产生重复 ID 时被拒绝。
#[test]
fn duplicate_shortcuts_are_rejected() {
    let mut config = Config::default();
    config.hotkeys.editor = "super+g".into();
    assert!(config.validate().is_err());
    config.hotkeys.editor = "g".into();
    assert!(config.validate().is_err());
}

/// 输入：无；返回：无，验证未知配置字段和无效模式不会悄悄降级。
#[test]
fn unknown_settings_are_rejected() {
    assert!(toml::from_str::<Config>("[editor]\nreuse_windw = false").is_err());
    let mut config = Config::default();
    config.terminal.alacritty_mode = "unknown".into();
    assert!(config.validate().is_err());
}

/// 输入：无；返回：无，验证局部配置兼容默认值并保留显式选项。
#[test]
fn partial_config_uses_defaults() {
    let config: Config = toml::from_str("[editor]\nreuse_window = false").unwrap();
    assert!(!config.editor.reuse_window);
    assert_eq!(config.hotkeys.terminal, "cmd+g");
    assert!(config.validate().is_ok());
}

/// 输入：无；返回：无，验证路径中的空白、引号和换行不会被 trim 破坏。
#[test]
fn finder_preserves_unusual_names() {
    let path = "/tmp/中文 ' \" $HOME\n folder \n";
    assert_eq!(
        finder::decode_path(&format!("{path}\n")).unwrap(),
        PathBuf::from(path)
    );
    assert_eq!(
        finder::decode_path("\n").unwrap(),
        dirs::home_dir().unwrap()
    );
}

/// 输入：无；返回：无，验证保存无效配置不会覆盖已有有效文件。
#[test]
fn invalid_save_preserves_existing_config() {
    let dir = std::env::temp_dir().join(format!("finderjump-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let mut config = Config::load(&path).unwrap();
    let previous = fs::read_to_string(&path).unwrap();
    config.hotkeys.terminal = "invalid+shortcut".into();
    assert!(config.save(&path).is_err());
    assert_eq!(previous, fs::read_to_string(&path).unwrap());
    fs::remove_dir_all(dir).unwrap();
}

/// 输入：无；返回：无，验证显式错误路径不会回退到其他已安装应用。
#[test]
fn invalid_executable_override_is_reported() {
    assert!(launcher::resolve(Some("/nonexistent/finderjump-test"), "code", "unused").is_err());
    assert!(launcher::resolve(Some("relative/path"), "code", "unused").is_err());
}

/// 输入：无；返回：无，验证脚本参数中的引号、美元符号和 Unicode 按原样返回。
#[test]
fn applescript_arguments_are_not_source_code() {
    let path = "/tmp/中文 ' \" $(echo unexpected) space ";
    let output = crate::process::applescript(
        "on run argv\nreturn item 1 of argv\nend run",
        &[std::ffi::OsStr::new(path)],
    )
    .unwrap();
    assert_eq!(finder::decode_path(&output).unwrap(), PathBuf::from(path));
}

/// 输入：无；返回：无，验证子进程非零退出不会被误报成功且包含诊断信息。
#[test]
fn child_failure_includes_stderr() {
    let error = crate::process::checked(
        std::process::Command::new("/bin/sh").args(["-c", "printf 'test failure' >&2; exit 7"]),
    )
    .unwrap_err();
    assert!(error.to_string().contains("test failure"));
}

/// 输入：无；返回：无，验证旧配置默认关闭日志且新开关可序列化保存。
#[test]
fn logging_setting_is_backward_compatible() {
    let mut config: Config = toml::from_str("[hotkeys]\nterminal = 'cmd+g'").unwrap();
    assert!(!config.logging.enabled);
    config.logging.enabled = true;
    let decoded: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
    assert!(decoded.logging.enabled);
    assert!(toml::from_str::<Config>("[logging]\nenable = true").is_err());
}

/// 输入：无；返回：无，验证关闭日志不创建文件，也不修改已有日志。
#[test]
fn logging_switch_controls_disk_writes() {
    let dir = std::env::temp_dir().join(format!("finderjump-log-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.toml");
    let log = dir.join("finderjump.log");
    crate::logging::write(&config, false, "disabled").unwrap();
    assert!(!log.exists());
    crate::logging::write(&config, true, "test diagnostic").unwrap();
    let previous = fs::read_to_string(&log).unwrap();
    assert!(previous.contains("test diagnostic"));
    crate::logging::write(&config, false, "should not appear").unwrap();
    assert_eq!(fs::read_to_string(&log).unwrap(), previous);
    fs::remove_dir_all(dir).unwrap();
}

/// 输入：无；返回：无，验证录制数字、方向键、功能键与配置解析一致。
#[test]
fn recorded_shortcuts_match_config_validation() {
    use gpui::Modifiers;
    let modifiers = Modifiers {
        platform: true,
        shift: true,
        ..Default::default()
    };
    for key in ["g", "1", "up", "f12", "space"] {
        let value = crate::settings::recorded_shortcut(key, modifiers).unwrap();
        let mut config = Config::default();
        config.hotkeys.terminal = value;
        assert!(config.validate().is_ok());
    }
    assert!(
        crate::settings::recorded_shortcut(
            "g",
            Modifiers {
                shift: true,
                ..Default::default()
            }
        )
        .is_none()
    );
    let mut config = Config::default();
    config.hotkeys.terminal = "shift+g".into();
    assert!(config.validate().is_err());
}
