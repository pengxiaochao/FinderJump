use crate::{app::App, autostart, config::Hotkeys};
use gpui::{
    Context, Div, KeyDownEvent, Modifiers, PathPromptOptions, Stateful, Window, WindowAppearance,
    div, prelude::*, rgb,
};

#[derive(Clone, Copy)]
enum Edit {
    Login,
    Logging,
    Logs,
    Terminal(bool),
    Mode(bool),
    Reuse,
    Record(bool),
    Defaults,
    ChoosePath(bool),
    ResetPath(bool),
    Save,
    Discard,
}

/// 输入：应用、窗口和上下文；返回：跟随系统主题的设置元素树。
pub fn draw(app: &App, window: &mut Window, cx: &mut Context<App>) -> impl IntoElement {
    let dark = matches!(
        window.appearance(),
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    div().flex().flex_col().size_full().p_6().gap_4()
        .bg(rgb(if dark { 0x202124 } else { 0xf4f5f7 }))
        .text_color(rgb(if dark { 0xf0f1f3 } else { 0x202124 })).text_sm()
        .track_focus(&app.focus)
        .on_key_down(cx.listener(capture))
        .child(div().text_2xl().font_weight(gpui::FontWeight::BOLD).child("FinderJump"))
        .child("Your Finder selection, one shortcut away.")
        .child(div().id("settings-scroll").flex_1().min_h_0().overflow_y_scroll()
            .child(div().flex().flex_col().gap_4().flex_shrink_0()
            .child(general(app, cx))
            .child(shortcuts(app, cx))
            .child(applications(app, cx))
            .child("First use: allow Finder / iTerm2 in System Settings → Privacy & Security → Automation.")))
        .child(footer(app, cx))
}

/// 输入：标题；返回：统一留白与边框的分组容器。
fn card(title: &'static str) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .border_1()
        .border_color(rgb(0x808080))
        .rounded_lg()
        .child(div().text_xs().child(title))
}

/// 输入：两侧元素；返回：可换行的水平设置行。
fn row(left: impl IntoElement, right: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_2()
        .child(left)
        .child(right)
}

/// 输入：按钮标识、标题、选中/可用状态、动作及上下文；返回：带主线程回调的按钮。
fn button(
    id: &'static str,
    label: impl Into<gpui::SharedString>,
    active: bool,
    enabled: bool,
    edit: Edit,
    cx: &Context<App>,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x808080))
        .when(active, |d| d.bg(rgb(0x007aff)).text_color(rgb(0xffffff)))
        .when(!enabled, |d| d.opacity(0.45))
        .child(label.into())
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(rgb(0x42678f)).text_color(rgb(0xffffff)))
                .on_click(cx.listener(move |app, _, window, cx| apply_edit(app, edit, window, cx)))
        })
}

/// 输入：应用及上下文；返回：登录启动与诊断日志设置。
fn general(app: &App, cx: &Context<App>) -> Div {
    card("GENERAL")
        .child(row(
            "Launch at login",
            button("login", toggle(app.login), app.login, true, Edit::Login, cx),
        ))
        .child("Applies immediately to your next login.")
        .child(row(
            "Enable diagnostic logging",
            button(
                "logging",
                toggle(app.draft.logging.enabled),
                app.draft.logging.enabled,
                true,
                Edit::Logging,
                cx,
            ),
        ))
        .child(row(
            "Save to apply; existing logs are kept when disabled.",
            button("logs", "Show Log…", false, true, Edit::Logs, cx),
        ))
}

/// 输入：布尔状态；返回：有明确文字含义的开关标签。
fn toggle(enabled: bool) -> &'static str {
    if enabled { "On" } else { "Off" }
}

/// 输入：应用及上下文；返回：录制按钮、快捷键校验信息及默认值入口。
fn shortcuts(app: &App, cx: &Context<App>) -> Div {
    let mut card = card("KEYBOARD SHORTCUTS")
        .child(row("Open Terminal", shortcut(app, true, cx)))
        .child(row("Open VS Code", shortcut(app, false, cx)))
        .child("Click a shortcut, then press Command, Control or Option with a key. Esc cancels.")
        .child(button(
            "defaults",
            "Restore default shortcuts",
            false,
            true,
            Edit::Defaults,
            cx,
        ));
    if let Err(error) = app.draft.validate() {
        card = card.child(div().text_color(rgb(0xf06060)).child(format!("{error:#}")));
    }
    card
}

/// 输入：应用及上下文；返回：Alacritty 打开方式和实验模式提示。
fn terminal_mode(app: &App, cx: &Context<App>) -> Div {
    let native = app.draft.terminal.alacritty_mode == "native_tab";
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(row(
            "Open using",
            div()
                .flex()
                .gap_2()
                .child(button("ipc", "IPC", !native, true, Edit::Mode(false), cx))
                .child(button(
                    "native",
                    "Native tab",
                    native,
                    true,
                    Edit::Mode(true),
                    cx,
                )),
        ))
        .when(native, |d| {
            d.child("Experimental · requires Accessibility permission.")
        })
}

/// 输入：应用、终端标记及上下文；返回：当前快捷键或正在录制的提示按钮。
fn shortcut(app: &App, terminal: bool, cx: &Context<App>) -> Stateful<Div> {
    let active = app.recording == Some(terminal);
    let value = if terminal {
        &app.draft.hotkeys.terminal
    } else {
        &app.draft.hotkeys.editor
    };
    let label = if active {
        "Type shortcut…".into()
    } else {
        display_shortcut(value)
    };
    button(
        if terminal {
            "terminal-key"
        } else {
            "editor-key"
        },
        label,
        active,
        true,
        Edit::Record(terminal),
        cx,
    )
}

/// 输入：应用及上下文；返回：终端类型、打开方式和编辑器选项。
fn applications(app: &App, cx: &Context<App>) -> Div {
    let alacritty = app.draft.terminal.kind == "alacritty";
    let mut card = card("APPLICATIONS").child(row(
        "Terminal",
        div()
            .flex()
            .gap_2()
            .child(button(
                "iterm",
                "iTerm2",
                !alacritty,
                true,
                Edit::Terminal(false),
                cx,
            ))
            .child(button(
                "alacritty",
                "Alacritty",
                alacritty,
                true,
                Edit::Terminal(true),
                cx,
            )),
    ));
    if alacritty {
        card = card
            .child(terminal_mode(app, cx))
            .child(executable(app, true, cx));
    }
    card.child(row(
        "VS Code · Reuse the last active window",
        button(
            "reuse",
            toggle(app.draft.editor.reuse_window),
            app.draft.editor.reuse_window,
            true,
            Edit::Reuse,
            cx,
        ),
    ))
    .child(executable(app, false, cx))
}

/// 输入：应用、终端标记及上下文；返回：可执行文件路径和原生选择/自动检测入口。
fn executable(app: &App, terminal: bool, cx: &Context<App>) -> Div {
    let value = if terminal {
        &app.draft.terminal.executable
    } else {
        &app.draft.editor.executable
    };
    let (label, choose_id, reset_id) = if terminal {
        ("Alacritty executable", "choose-terminal", "reset-terminal")
    } else {
        ("VS Code executable", "choose-editor", "reset-editor")
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(label)
        .child(
            div().overflow_x_hidden().child(
                value
                    .clone()
                    .unwrap_or_else(|| "Automatic detection".into()),
            ),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(button(
                    choose_id,
                    "Choose…",
                    false,
                    true,
                    Edit::ChoosePath(terminal),
                    cx,
                ))
                .child(button(
                    reset_id,
                    "Automatic",
                    false,
                    value.is_some(),
                    Edit::ResetPath(terminal),
                    cx,
                )),
        )
}

/// 输入：应用和上下文；返回：固定底部保存、放弃、进度及结果信息。
fn footer(app: &App, cx: &Context<App>) -> Div {
    let dirty = app.draft != app.config;
    let valid = app.draft.validate().is_ok() && app.recording.is_none();
    let mut footer = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(if dirty {
            "Unsaved changes"
        } else {
            "Settings up to date"
        })
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .child(button(
                    "discard",
                    "Discard changes",
                    false,
                    dirty || app.recording.is_some(),
                    Edit::Discard,
                    cx,
                ))
                .child(button(
                    "save",
                    "Save & Apply",
                    true,
                    dirty && valid,
                    Edit::Save,
                    cx,
                )),
        );
    if app.busy {
        footer = footer.child("Opening Finder target…");
    }
    if !app.message.is_empty() {
        footer = footer.child(
            div()
                .when(app.failed, |d| d.text_color(rgb(0xf06060)))
                .child(app.message.clone()),
        );
    }
    footer
}

/// 输入：应用、编辑动作、窗口及上下文；返回：无，更新草稿或执行设置操作并通知重绘。
fn apply_edit(app: &mut App, edit: Edit, window: &mut Window, cx: &mut Context<App>) {
    let result = match edit {
        Edit::Login => {
            let result = autostart::set_enabled(!app.login);
            app.login = autostart::enabled();
            Some(result)
        }
        Edit::Logs => Some(app.open_logs()),
        Edit::ChoosePath(terminal) => {
            choose_path(terminal, cx);
            None
        }
        Edit::Save => Some(app.apply(app.draft.clone())),
        _ => {
            edit_draft(app, edit);
            window.focus(&app.focus);
            None
        }
    };
    if let Some(result) = result {
        app.report(result, window, cx);
    }
    cx.notify();
}

/// 输入：应用与纯草稿编辑动作；返回：无，仅改变尚未保存的设置。
fn edit_draft(app: &mut App, edit: Edit) {
    match edit {
        Edit::Logging => app.draft.logging.enabled = !app.draft.logging.enabled,
        Edit::Terminal(value) => {
            app.draft.terminal.kind = if value { "alacritty" } else { "iterm2" }.into()
        }
        Edit::Mode(value) => {
            app.draft.terminal.alacritty_mode = if value { "native_tab" } else { "ipc" }.into()
        }
        Edit::Reuse => app.draft.editor.reuse_window = !app.draft.editor.reuse_window,
        Edit::Record(terminal) => {
            app.recording = if app.recording == Some(terminal) {
                None
            } else {
                Some(terminal)
            }
        }
        Edit::Defaults => {
            app.draft.hotkeys = Hotkeys::default();
            app.recording = None;
        }
        Edit::ResetPath(terminal) => {
            if terminal {
                app.draft.terminal.executable = None;
            } else {
                app.draft.editor.executable = None;
            }
        }
        Edit::Discard => {
            app.draft = app.config.clone();
            app.recording = None;
            app.message.clear();
            app.failed = false;
        }
        _ => {}
    }
}

/// 输入：终端标记和上下文；返回：无，异步选择可执行文件，取消时保持草稿不变。
fn choose_path(terminal: bool, cx: &mut Context<App>) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Choose executable".into()),
    });
    cx.spawn(async move |app, cx| {
        let result = paths.await;
        let _ = app.update(cx, |app, cx| {
            match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.first() {
                        let value = Some(path.to_string_lossy().into_owned());
                        if terminal {
                            app.draft.terminal.executable = value;
                        } else {
                            app.draft.editor.executable = value;
                        }
                    }
                }
                Ok(Ok(None)) => {}
                other => {
                    app.failed = true;
                    app.message = format!("Cannot choose executable: {other:?}");
                }
            }
            cx.notify();
        });
    })
    .detach();
}

/// 输入：可解析的快捷键字符串；返回：使用 macOS 修饰键符号的显示文本。
pub(crate) fn display_shortcut(value: &str) -> String {
    value
        .split('+')
        .map(|part| match part.to_lowercase().as_str() {
            "cmd" | "command" | "super" => "⌘".into(),
            "ctrl" | "control" => "⌃".into(),
            "alt" | "option" => "⌥".into(),
            "shift" => "⇧".into(),
            _ => part.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 输入：GPUI 键名及修饰键；返回：通过全局快捷键解析的字符串，不支持的按键返回空。
pub(crate) fn recorded_shortcut(key: &str, modifiers: Modifiers) -> Option<String> {
    if !(modifiers.platform || modifiers.control || modifiers.alt) {
        return None;
    }
    let mut parts = Vec::new();
    for (enabled, name) in [
        (modifiers.platform, "cmd"),
        (modifiers.control, "ctrl"),
        (modifiers.alt, "alt"),
        (modifiers.shift, "shift"),
    ] {
        if enabled {
            parts.push(name.to_owned());
        }
    }
    let key = match key {
        "up" => "arrowup",
        "down" => "arrowdown",
        "left" => "arrowleft",
        "right" => "arrowright",
        "enter" => "enter",
        "backspace" => "backspace",
        other => other,
    };
    parts.push(key.to_lowercase());
    let value = parts.join("+");
    value
        .parse::<global_hotkey::hotkey::HotKey>()
        .ok()
        .map(|_| value)
}

/// 输入：应用、按键事件、窗口及上下文；返回：无，录制组合键，Escape 取消并阻止事件继续传播。
fn capture(app: &mut App, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<App>) {
    let Some(terminal) = app.recording else {
        return;
    };
    cx.stop_propagation();
    if event.keystroke.key == "escape" {
        app.recording = None;
    } else if !event.is_held
        && let Some(value) = recorded_shortcut(&event.keystroke.key, event.keystroke.modifiers)
    {
        if terminal {
            app.draft.hotkeys.terminal = value;
        } else {
            app.draft.hotkeys.editor = value;
        }
        app.recording = None;
    }
    cx.notify();
}
