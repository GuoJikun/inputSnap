use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, REG_VALUE_TYPE,
};
use windows::Win32::Foundation::ERROR_SUCCESS;

use crate::tsf::InputMethod;

const SUB_KEY: &str = "Software\\KeepAppInputStatus";

/// 打开或创建注册表项
fn open_key(desired_access: u32) -> Result<windows::Win32::System::Registry::HKEY, String> {
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
        let wide: Vec<u16> = SUB_KEY.encode_utf16().chain(std::iter::once(0)).collect();

        // 先尝试打开
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(wide.as_ptr()),
            Some(0),
            windows::Win32::System::Registry::REG_SAM_FLAGS(desired_access),
            &mut hkey,
        );
        if result == ERROR_SUCCESS {
            return Ok(hkey);
        }

        // 打开失败（如键不存在）时创建，KEY_WRITE 下创建成功即可返回
        if desired_access == KEY_WRITE.0 {
            let mut disposition = windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION(0);
            let create_result = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                windows::core::PCWSTR(wide.as_ptr()),
                None,
                windows::core::PCWSTR(std::ptr::null()),
                windows::Win32::System::Registry::REG_OPTION_NON_VOLATILE,
                windows::Win32::System::Registry::REG_SAM_FLAGS(KEY_WRITE.0),
                None,
                &mut hkey,
                Some(&mut disposition),
            );
            if create_result == ERROR_SUCCESS {
                return Ok(hkey);
            }
            return Err(format!(
                "RegCreateKeyExW failed: {:?}, open: {:?}",
                create_result, result
            ));
        }

        Err(format!("RegOpenKeyExW failed: {:?}", result))
    }
}

/// 读取字符串值
fn read_string_value(hkey: windows::Win32::System::Registry::HKEY, name: &str) -> Option<String> {
    unsafe {
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut buf = [0u16; 512];
        let mut buf_size = (buf.len() * 2) as u32;
        let mut reg_type = REG_VALUE_TYPE(0);

        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_wide.as_ptr()),
            None,
            Some(&mut reg_type),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_size),
        );

        if result == ERROR_SUCCESS && reg_type.0 == REG_SZ.0 {
            let len = (buf_size / 2) as usize;
            let s = String::from_utf16_lossy(&buf[..len]);
            Some(s)
        } else {
            None
        }
    }
}

/// 写入字符串值
fn write_string_value(
    hkey: windows::Win32::System::Registry::HKEY,
    name: &str,
    value: &str,
) -> Result<(), String> {
    unsafe {
        let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let data = value_wide.as_ptr() as *const u8;
        let data_size = (value_wide.len() * 2) as u32;

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_wide.as_ptr()),
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(data, data_size as usize)),
        );

        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("RegSetValueExW failed: {:?}", result))
        }
    }
}

/// 保存进程的输入法配置（TSF Profile + HKL 兜底）
pub fn save_ime_for_process(process_name: &str, im: &InputMethod) -> Result<(), String> {
    let hkey = open_key(KEY_WRITE.0)?;
    let value = im.to_registry_string();
    let result = write_string_value(hkey, process_name, &value);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result
}

/// 读取进程保存的输入法配置
pub fn load_ime_for_process(process_name: &str) -> Option<InputMethod> {
    let hkey = open_key(KEY_READ.0).ok()?;
    let value = read_string_value(hkey, process_name);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    value.and_then(|s| InputMethod::from_registry_string(&s))
}

// ─── 开机自启 ────────────────────────────────────────────────────────

/// Windows 开机自启注册表路径
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
/// 自启动值名称
const AUTO_START_VALUE_NAME: &str = "InputSnap";
/// 标记用户是否已手动配置过自启（避免每次启动都强制启用）
const CONFIGURED_FLAG: &str = "AutoStartConfigured";

/// 检查是否已设置开机自启
pub fn is_auto_start_enabled() -> bool {
    unsafe {
        let run_wide: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(run_wide.as_ptr()),
            Some(0),
            windows::Win32::System::Registry::REG_SAM_FLAGS(KEY_READ.0),
            &mut hkey,
        );
        if result != ERROR_SUCCESS {
            return false;
        }

        let name_wide: Vec<u16> = AUTO_START_VALUE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut buf = [0u16; 512];
        let mut buf_size = (buf.len() * 2) as u32;
        let mut reg_type = REG_VALUE_TYPE(0);

        let result = RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(name_wide.as_ptr()),
            None,
            Some(&mut reg_type),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_size),
        );

        let _ = RegCloseKey(hkey);
        result == ERROR_SUCCESS
    }
}

/// 启用开机自启（写入当前 exe 路径到 Run 键）
pub fn enable_auto_start() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取 exe 路径失败: {}", e))?
        .to_string_lossy()
        .to_string();

    unsafe {
        let run_wide: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
        let mut disposition = windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION(0);

        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(run_wide.as_ptr()),
            None,
            windows::core::PCWSTR(std::ptr::null()),
            windows::Win32::System::Registry::REG_OPTION_NON_VOLATILE,
            windows::Win32::System::Registry::REG_SAM_FLAGS(KEY_WRITE.0),
            None,
            &mut hkey,
            Some(&mut disposition),
        );
        if result != ERROR_SUCCESS {
            return Err(format!("打开 Run 键失败: {:?}", result));
        }

        let name_wide: Vec<u16> = AUTO_START_VALUE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let value_wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();

        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(name_wide.as_ptr()),
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(
                value_wide.as_ptr() as *const u8,
                value_wide.len() * 2,
            )),
        );

        let _ = RegCloseKey(hkey);

        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("写入自启动值失败: {:?}", result))
        }
    }
}

/// 禁用开机自启（从 Run 键中删除对应值）
pub fn disable_auto_start() -> Result<(), String> {
    unsafe {
        let run_wide: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(run_wide.as_ptr()),
            Some(0),
            windows::Win32::System::Registry::REG_SAM_FLAGS(KEY_WRITE.0),
            &mut hkey,
        );
        if result != ERROR_SUCCESS {
            // Run 键不存在，等同于已禁用
            return Ok(());
        }

        let name_wide: Vec<u16> = AUTO_START_VALUE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let result = RegDeleteValueW(
            hkey,
            windows::core::PCWSTR(name_wide.as_ptr()),
        );

        let _ = RegCloseKey(hkey);

        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("删除自启动值失败: {:?}", result))
        }
    }
}

/// 检查用户是否已手动配置过自启（用于区分「从未配置」和「用户主动关闭」）
pub fn is_auto_start_configured() -> bool {
    let hkey = match open_key(KEY_READ.0) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let result = read_string_value(hkey, CONFIGURED_FLAG).is_some();
    unsafe { let _ = RegCloseKey(hkey); }
    result
}

/// 标记用户已手动配置过自启
pub fn mark_auto_start_configured() {
    let hkey = match open_key(KEY_WRITE.0) {
        Ok(k) => k,
        Err(e) => {
            log::warn!("打开注册表失败，无法标记自启配置: {}", e);
            return;
        }
    };
    let _ = write_string_value(hkey, CONFIGURED_FLAG, "1");
    unsafe { let _ = RegCloseKey(hkey); }
}
