use std::sync::atomic::{AtomicBool, Ordering};

/// 全局运行时状态
pub struct AppState {
    /// 是否启用自动切换
    pub enabled: AtomicBool,
    /// 上一次的前台窗口进程名
    pub last_process: std::sync::Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            last_process: std::sync::Mutex::new(None),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn toggle_enabled(&self) -> bool {
        let current = self.enabled.load(Ordering::Relaxed);
        let new = !current;
        self.enabled.store(new, Ordering::Relaxed);
        new
    }

    pub fn get_last_process(&self) -> Option<String> {
        self.last_process.lock().unwrap().clone()
    }

    pub fn set_last_process(&self, name: Option<String>) {
        *self.last_process.lock().unwrap() = name;
    }
}
