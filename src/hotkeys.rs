use crate::config::Config;
use anyhow::{Context, Result};
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    pub keys: [HotKey; 2],
}

impl Hotkeys {
    /// 输入：已验证配置；返回：持有已注册全局快捷键的管理器。
    pub fn new(config: &Config) -> Result<Self> {
        let keys = config.validate()?;
        let manager = GlobalHotKeyManager::new()?;
        manager
            .register(keys[0])
            .context("Register terminal shortcut")?;
        if let Err(error) = manager.register(keys[1]) {
            let _ = manager.unregister(keys[0]);
            return Err(error).context("Register editor shortcut");
        }
        Ok(Self { manager, keys })
    }

    /// 输入：新配置；返回：更新结果；仅注册新增项，失败时删除新增项并保留旧键。
    pub fn replace(&mut self, config: &Config) -> Result<()> {
        let next = config.validate()?;
        let additions: Vec<_> = next
            .iter()
            .filter(|k| !self.keys.contains(k))
            .copied()
            .collect();
        let mut registered = Vec::new();
        for key in additions {
            if let Err(error) = self.manager.register(key) {
                for new in registered {
                    self.manager.unregister(new)?;
                }
                return Err(error).context("Shortcut is unavailable; previous shortcuts retained");
            }
            registered.push(key);
        }
        let mut removed = Vec::new();
        for old in self.keys.iter().filter(|k| !next.contains(k)) {
            if let Err(error) = self.manager.unregister(*old) {
                for key in removed {
                    self.manager.register(key)?;
                }
                for key in registered {
                    self.manager.unregister(key)?;
                }
                return Err(error).context("Unable to replace shortcuts");
            }
            removed.push(*old);
        }
        self.keys = next;
        Ok(())
    }
}
