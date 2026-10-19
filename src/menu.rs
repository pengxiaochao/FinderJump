use anyhow::Result;
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

/// 输入：无；返回：菜单栏图标，菜单事件通过固定 ID 分派。
pub fn create() -> Result<TrayIcon> {
    let menu = Menu::new();
    for (id, label) in [
        ("terminal", "Open Terminal"),
        ("editor", "Open VS Code"),
        ("preferences", "Preferences…"),
        ("reload", "Reload Config"),
        ("config", "Open Config File"),
        ("logs", "Show Log"),
    ] {
        menu.append(&MenuItem::with_id(id, label, true, None))?;
    }
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id("quit", "Quit FinderJump", true, None))?;
    Ok(TrayIconBuilder::new()
        .with_title("⌘↗")
        .with_tooltip("FinderJump")
        .with_menu(Box::new(menu))
        .build()?)
}

/// 输入：无，必须位于 GPUI 启动后的主线程；返回：切换菜单栏应用策略的结果。
#[cfg(target_os = "macos")]
pub fn hide_dock_icon() -> Result<()> {
    use anyhow::{Context, ensure};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let marker = MainThreadMarker::new().context("Dock policy requires the main thread")?;
    let application = NSApplication::sharedApplication(marker);
    ensure!(
        application.setActivationPolicy(NSApplicationActivationPolicy::Accessory),
        "Cannot hide FinderJump Dock icon"
    );
    Ok(())
}
