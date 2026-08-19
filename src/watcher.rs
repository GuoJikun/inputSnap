use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::config::AppState;
use crate::ime::{
    activate_input_method, get_foreground_input_method, get_foreground_process_name,
};
use crate::registry::{load_ime_for_process, save_ime_for_process};
use crate::tsf;

type HHook = *mut core::ffi::c_void;

unsafe extern "system" {
    fn SetWinEventHook(
        eventmin: u32,
        eventmax: u32,
        hmod: *mut core::ffi::c_void,
        pfn: *const core::ffi::c_void,
        idprocess: u32,
        idthread: u32,
        dwflags: u32,
    ) -> HHook;
    fn UnhookWinEvent(hwineventhook: HHook) -> i32;
    fn GetLastError() -> u32;
    fn PeekMessageW(
        lpmsg: *mut core::ffi::c_void,
        hWnd: *mut core::ffi::c_void,
        wMsgFilterMin: u32,
        wMsgFilterMax: u32,
        wRemoveMsg: u32,
    ) -> i32;
}

pub struct SafeHook(HHook);
unsafe impl Send for SafeHook {}
unsafe impl Sync for SafeHook {}

impl Drop for SafeHook {
    fn drop(&mut self) {
        unsafe { UnhookWinEvent(self.0); }
    }
}

static GLOBAL_STATE: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// 保存全局状态指针。`Arc::into_raw` 返回指向 `AppState` 本身的 data 指针，
/// 同时泄漏该 Arc（引用计数不减），保证回调中可安全使用 `&'static AppState`。
pub fn set_state_ptr(state: Arc<AppState>) {
    let raw = Arc::into_raw(state) as *mut ();
    GLOBAL_STATE.store(raw, Ordering::Release);
}

fn get_global_state() -> Option<&'static AppState> {
    let ptr = GLOBAL_STATE.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        // 注意：GLOBAL_STATE 存的是 Arc 的 data 指针（指向 AppState），
        // 不能转成 &Arc<AppState>，否则回调中解引用会读到垃圾指针导致访问冲突
        Some(unsafe { &*(ptr as *const AppState) })
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn event_callback(
    _hook: HHook,
    event: u32,
    _hwnd: *mut core::ffi::c_void,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    _event_time: u32,
) {
    if event != 0x0003 {
        return;
    }

    let state = match get_global_state() {
        Some(s) => s,
        None => return,
    };

    if !state.is_enabled() {
        return;
    }

    let process_name = match get_foreground_process_name() {
        Some(name) => name,
        None => return,
    };

    if process_name.is_empty() {
        return;
    }

    let current_im = match get_foreground_input_method() {
        Some(im) => im,
        None => return,
    };

    if let Some(ref last) = state.get_last_process() {
        if last == &process_name {
            return;
        }
    }

    log::debug!("前台窗口切换到: {}", process_name);

    if let Some(saved_im) = load_ime_for_process(&process_name) {
        if !saved_im.same_profile(&current_im) {
            log::info!(
                "恢复 {} 的输入法: {:?} -> {:?}",
                process_name,
                current_im.profile,
                saved_im.profile
            );
            if !activate_input_method(&saved_im) {
                log::warn!("激活输入法失败: {}", process_name);
            }
        }
    } else {
        log::info!(
            "保存 {} 的输入法: lang={:04X}, profile={:?}",
            process_name,
            current_im.lang_id,
            current_im.profile
        );
        if let Err(e) = save_ime_for_process(&process_name, &current_im) {
            log::error!("保存输入法失败 {}: {}", process_name, e);
        }
    }

    state.set_last_process(Some(process_name));
}

pub fn start_watching(_state: Arc<AppState>) -> Result<SafeHook, String> {
    // 初始化 TSF（COM + Profile Manager）
    if let Err(e) = tsf::init_tsf() {
        log::warn!("TSF 初始化失败，将使用 HKL 兜底: {}", e);
    }

    unsafe {
        log::info!("调用 SetWinEventHook...");

        // 创建线程消息队列，否则 SetWinEventHook 会因 ERROR_INVALID_MESSAGE(1426) 失败
        let mut dummy_msg: [u8; 64] = [0; 64];
        PeekMessageW(
            dummy_msg.as_mut_ptr() as *mut core::ffi::c_void,
            std::ptr::null_mut(),
            0,
            0,
            0,
        );

        let cb_ptr = event_callback as *const core::ffi::c_void;
        log::info!("回调地址: {:?}", cb_ptr);

        let hook = SetWinEventHook(
            0x0003,
            0x0003,
            std::ptr::null_mut(),
            cb_ptr,
            0,
            0,
            0x0000,
        );

        log::info!("返回值: {:?}", hook);

        if hook.is_null() {
            let err = GetLastError();
            return Err(format!("SetWinEventHook 失败, error={}", err));
        }

        log::info!("窗口监听启动成功");
        Ok(SafeHook(hook))
    }
}
