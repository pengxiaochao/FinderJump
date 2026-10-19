use crate::{
    autostart,
    config::Config,
    hotkeys::Hotkeys,
    launcher::{self, Action},
    logging, menu, process, settings,
};
use anyhow::Result;
use async_channel::Sender;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use gpui::{
    Application, Bounds, Context, FocusHandle, Window, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use std::{path::PathBuf, process::Command, thread};
use tray_icon::{TrayIcon, menu::MenuEvent};

pub enum Event {
    Hotkey(u32),
    Menu(String),
    Finished(Result<()>),
}
pub struct App {
    pub config: Config,
    pub draft: Config,
    pub message: String,
    pub recording: Option<bool>,
    pub login: bool,
    pub busy: bool,
    pub failed: bool,
    path: PathBuf,
    hotkeys: Hotkeys,
    sender: Sender<Event>,
    _tray: TrayIcon,
    pub focus: FocusHandle,
}

impl App {
    /// 输入：配置、路径、快捷键、托盘、窗口和 GPUI 上下文；返回：持有菜单、快捷键和事件订阅的应用。
    pub fn new(
        config: Config,
        path: PathBuf,
        hotkeys: Hotkeys,
        tray: TrayIcon,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (sender, events) = async_channel::unbounded();
        connect_events(&sender);
        cx.spawn_in(window, async move |app, cx| {
            while let Ok(event) = events.recv().await {
                if app
                    .update_in(cx, |app, window, cx| app.receive(event, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let focus = configure_window(window, cx);
        Self {
            draft: config.clone(),
            hotkeys,
            config,
            path,
            sender,
            _tray: tray,
            message: String::new(),
            recording: None,
            login: autostart::enabled(),
            busy: false,
            failed: false,
            focus,
        }
    }

    /// 输入：新配置；返回：快捷键和磁盘配置事务更新结果，写盘失败回滚快捷键。
    pub fn apply(&mut self, next: Config) -> Result<()> {
        self.hotkeys.replace(&next)?;
        if let Err(error) = next.save(&self.path) {
            self.hotkeys.replace(&self.config)?;
            return Err(error);
        }
        self.config = next.clone();
        self.draft = next;
        Ok(())
    }

    /// 输入：操作结果、窗口和 GPUI 上下文；返回：无，更新状态并持久化错误，错误时显示设置窗口。
    pub fn report(&mut self, result: Result<()>, window: &mut Window, cx: &mut Context<Self>) {
        self.failed = result.is_err();
        self.message = match result {
            Ok(()) => "Operation completed".into(),
            Err(error) => {
                show(window, cx);
                format!("{error:#}")
            }
        };
        if let Err(error) = logging::write(&self.path, self.config.logging.enabled, &self.message) {
            self.failed = true;
            self.message
                .push_str(&format!(" · Cannot write log: {error}"));
        }
    }

    /// 输入：当前配置路径；返回：在系统文本编辑器中打开日志的结果。
    pub fn open_logs(&self) -> Result<()> {
        let path = logging::prepare(&self.path)?;
        process::checked(Command::new("/usr/bin/open").arg("-t").arg(path)).map(|_| ())
    }

    /// 输入：启动动作；返回：无，单工作线程执行，忙碌时忽略重复触发。
    fn launch(&mut self, action: Action) {
        if self.busy || self.recording.is_some() {
            return;
        }
        self.busy = true;
        let config = self.config.clone();
        let sender = self.sender.clone();
        thread::spawn(move || {
            let _ = sender.send_blocking(Event::Finished(launcher::execute(action, &config)));
        });
    }

    /// 输入：菜单 ID 和 UI 上下文；返回：无，分派打开、重载及退出操作。
    fn on_menu(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let result = match id {
            "terminal" => {
                self.launch(Action::Terminal);
                return;
            }
            "editor" => {
                self.launch(Action::Editor);
                return;
            }
            "preferences" => {
                show(window, cx);
                return;
            }
            "quit" => {
                cx.quit();
                return;
            }
            "reload" => Config::load(&self.path).and_then(|cfg| self.apply(cfg)),
            "config" => process::checked(Command::new("/usr/bin/open").arg("-t").arg(&self.path))
                .map(|_| ()),
            "logs" => self.open_logs(),
            _ => return,
        };
        self.report(result, window, cx);
    }

    /// 输入：全局快捷键 ID；返回：无，录制已有快捷键或执行动作。
    fn on_hotkey(&mut self, id: u32) {
        let action = if id == self.hotkeys.keys[0].id() {
            Action::Terminal
        } else if id == self.hotkeys.keys[1].id() {
            Action::Editor
        } else {
            return;
        };
        if let Some(terminal) = self.recording {
            let value = match action {
                Action::Terminal => self.config.hotkeys.terminal.clone(),
                Action::Editor => self.config.hotkeys.editor.clone(),
            };
            if terminal {
                self.draft.hotkeys.terminal = value;
            } else {
                self.draft.hotkeys.editor = value;
            }
            self.recording = None;
        } else {
            self.launch(action);
        }
    }

    /// 输入：原生事件、窗口及上下文；返回：无，在主线程消费单个事件并通知重绘。
    fn receive(&mut self, event: Event, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            Event::Hotkey(id) => self.on_hotkey(id),
            Event::Menu(id) => self.on_menu(&id, window, cx),
            Event::Finished(result) => {
                self.busy = false;
                let succeeded = result.is_ok();
                self.report(result, window, cx);
                if succeeded && !self.failed {
                    self.message = "Finder target opened".into();
                }
            }
        }
        cx.notify();
    }
}

impl Render for App {
    /// 输入：窗口及 GPUI 上下文；返回：完整设置界面元素树。
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        settings::draw(self, window, cx)
    }
}

/// 输入：窗口及视图上下文；返回：根焦点句柄，关闭或失焦时取消快捷键录制。
fn configure_window(window: &mut Window, cx: &mut Context<App>) -> FocusHandle {
    cx.observe_window_appearance(window, |_, _, cx| cx.notify())
        .detach();
    cx.observe_window_activation(window, |app, window, cx| {
        if !window.is_window_active() {
            app.recording = None;
            cx.notify();
        }
    })
    .detach();
    let app = cx.weak_entity();
    window.on_window_should_close(cx, move |_, cx| {
        let _ = app.update(cx, |app, cx| {
            app.recording = None;
            cx.notify();
        });
        cx.hide();
        false
    });
    let focus = cx.focus_handle();
    window.focus(&focus);
    focus
}

/// 输入：窗口及应用上下文；返回：无，重新显示并聚焦隐藏的设置窗口。
fn show(window: &mut Window, cx: &mut gpui::App) {
    cx.activate(true);
    window.activate_window();
}

/// 输入：事件发送端；返回：无，通过异步通道唤醒 GPUI，不进行定时轮询。
fn connect_events(sender: &Sender<Event>) {
    let hotkey_sender = sender.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            let _ = hotkey_sender.try_send(Event::Hotkey(event.id));
        }
    }));
    let menu_sender = sender.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = menu_sender.try_send(Event::Menu(event.id.0));
    }));
}

/// 输入：配置、配置路径与初始可见状态；返回：GPUI 主循环的启动结果。
pub fn run(config: Config, path: PathBuf, visible: bool) -> Result<()> {
    let error = std::rc::Rc::new(std::cell::RefCell::new(None));
    let startup_error = error.clone();
    Application::new().run(move |cx| {
        if let Err(failure) = open(config, path, visible, cx) {
            *startup_error.borrow_mut() = Some(failure);
            cx.quit();
        }
    });
    match error.take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// 输入：配置、路径、可见性与应用上下文；返回：创建设置窗口及原生资源的结果。
fn open(config: Config, path: PathBuf, visible: bool, cx: &mut gpui::App) -> Result<()> {
    #[cfg(target_os = "macos")]
    menu::hide_dock_icon()?;
    let bounds = Bounds::centered(None, size(px(620.), px(760.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(540.), px(520.))),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("FinderJump Preferences".into()),
            ..Default::default()
        }),
        show: visible,
        focus: visible,
        ..Default::default()
    };
    let hotkeys = Hotkeys::new(&config)?;
    let tray = menu::create()?;
    cx.open_window(options, |window, cx| {
        cx.new(|cx| App::new(config, path, hotkeys, tray, window, cx))
    })?;
    if visible {
        cx.activate(true);
    }
    Ok(())
}
