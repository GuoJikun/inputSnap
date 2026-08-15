use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, KLF_SETFORPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};

// Re-export HKL for use by other modules
pub use windows::Win32::UI::Input::KeyboardAndMouse::HKL;

/// 获取前台窗口的 HKL（键盘布局句柄）
pub fn get_foreground_hkl() -> Option<HKL> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut process_id = 0u32;
        let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if thread_id == 0 {
            return None;
        }
        let hkl = GetKeyboardLayout(thread_id);
        Some(hkl)
    }
}

/// 获取前台窗口的进程 ID
pub fn get_foreground_process_id() -> Option<u32> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == 0 {
            None
        } else {
            Some(process_id)
        }
    }
}

/// 激活指定的键盘布局
pub fn activate_keyboard_layout(hkl: HKL) -> bool {
    unsafe { ActivateKeyboardLayout(hkl, KLF_SETFORPROCESS).is_ok() }
}

/// 将 HKL 值转换为可读的字符串（例如 "0x08040804"）
pub fn hkl_to_string(hkl: HKL) -> String {
    format!("0x{:08X}", hkl.0 as usize)
}

/// 从字符串解析 HKL 值
pub fn string_to_hkl(s: &str) -> Option<HKL> {
    let s = s.trim();
    let val = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()?
    } else {
        s.parse::<usize>().ok()?
    };
    Some(HKL(val as *mut core::ffi::c_void))
}

/// 获取前台窗口进程名
pub fn get_foreground_process_name() -> Option<String> {
    let pid = get_foreground_process_id()?;
    get_process_name_by_pid(pid)
}

/// 根据进程 ID 获取进程名
pub fn get_process_name_by_pid(pid: u32) -> Option<String> {
    unsafe {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        use windows::Win32::System::ProcessStatus::GetProcessImageFileNameW;

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let len = GetProcessImageFileNameW(handle, &mut buf);
        let _ = CloseHandle(handle);

        if len == 0 {
            return None;
        }

        // 防止 API 返回长度超出缓冲区导致切片越界
        let len = len.min(buf.len() as u32);

        let path = OsString::from_wide(&buf[..len as usize]);
        let path_str = path.to_string_lossy();
        // 取最后的文件名部分
        let name = path_str
            .rsplit_once('\\')
            .map(|(_, n)| n)
            .unwrap_or(&path_str);
        Some(name.to_lowercase())
    }
}
