use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};

// Re-export TSF 类型供其他模块使用
pub use crate::tsf::InputMethod;

/// 获取前台窗口的输入法状态（通过 TSF Profile，兜底 GetKeyboardLayout）
pub fn get_foreground_input_method() -> Option<InputMethod> {
    crate::tsf::get_foreground_input_method()
}

/// 激活指定的输入法（通过 TSF，兜底 ActivateKeyboardLayout）
pub fn activate_input_method(im: &InputMethod) -> bool {
    crate::tsf::activate_input_method(im)
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
