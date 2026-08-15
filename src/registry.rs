use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, REG_VALUE_TYPE,
};
use windows::Win32::Foundation::ERROR_SUCCESS;

use crate::ime::{hkl_to_string, string_to_hkl, HKL};

const SUB_KEY: &str = "Software\\KeepAppInputStatus";

/// 打开或创建注册表项
fn open_key(desired_access: u32) -> Result<windows::Win32::System::Registry::HKEY, String> {
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY(std::ptr::null_mut());
        let wide: Vec<u16> = SUB_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(wide.as_ptr()),
            Some(0),
            windows::Win32::System::Registry::REG_SAM_FLAGS(desired_access),
            &mut hkey,
        );
        if result == ERROR_SUCCESS {
            Ok(hkey)
        } else {
            Err(format!("RegOpenKeyExW failed: {:?}", result))
        }
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

/// 保存进程的输入法 HKL 值
pub fn save_ime_for_process(process_name: &str, hkl: HKL) -> Result<(), String> {
    let hkey = open_key(KEY_WRITE.0)?;
    let value = hkl_to_string(hkl);
    let result = write_string_value(hkey, process_name, &value);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    result
}

/// 读取进程保存的输入法 HKL 值
pub fn load_ime_for_process(process_name: &str) -> Option<HKL> {
    let hkey = open_key(KEY_READ.0).ok()?;
    let value = read_string_value(hkey, process_name);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    value.and_then(|s| string_to_hkl(&s))
}
