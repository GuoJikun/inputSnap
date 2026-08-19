// TSF 封装层：获取和设置输入法 Profile
// 替代 GetKeyboardLayout，解决 Windows 11 上 HKL 无法区分输入法模式的问题

use windows::core::GUID;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ActivateKeyboardLayout, GetKeyboardLayout, KLF_SETFORPROCESS,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_InputProcessorProfiles, GUID_TFCAT_TIP_KEYBOARD,
    ITfInputProcessorProfileMgr, TF_INPUTPROCESSORPROFILE,
    TF_PROFILETYPE_INPUTPROCESSOR,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// 输入法状态标识
#[derive(Clone, Debug)]
pub struct InputMethod {
    /// 语言 ID（如 0x0804 = 简体中文）
    pub lang_id: u16,
    /// TSF Profile GUID（区分同一输入法不同模式）
    pub profile: GUID,
    /// Profile 类型（1=InputProcessor, 2=KeyboardLayout）
    pub profile_type: u32,
    /// HKL 值（兜底用）
    pub hkl: usize,
}

impl InputMethod {
    /// 判断两个 InputMethod 是否代表相同的输入法配置
    pub fn same_profile(&self, other: &InputMethod) -> bool {
        // 如果 Profile GUID 非零，优先比较 GUID + 语言 ID
        if self.profile != GUID::zeroed() && other.profile != GUID::zeroed() {
            return self.profile == other.profile && self.lang_id == other.lang_id;
        }
        // 兜底：比较 HKL
        self.hkl == other.hkl
    }

    /// 序列化为注册表存储格式
    pub fn to_registry_string(&self) -> String {
        format!(
            "{:04X}|{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}|0x{:08X}",
            self.lang_id,
            self.profile.data1,
            self.profile.data2,
            self.profile.data3,
            self.profile.data4[0],
            self.profile.data4[1],
            self.profile.data4[2],
            self.profile.data4[3],
            self.profile.data4[4],
            self.profile.data4[5],
            self.profile.data4[6],
            self.profile.data4[7],
            self.hkl,
        )
    }

    /// 从字符串反序列化（兼容旧格式和新格式）
    pub fn from_registry_string(s: &str) -> Option<Self> {
        let s = s.trim();

        // 新格式: "0804|{GUID}|0x08040804"
        if s.contains('|') {
            let parts: Vec<&str> = s.split('|').collect();
            if parts.len() == 3 {
                let lang_id = u16::from_str_radix(parts[0], 16).ok()?;
                let profile = parse_guid(parts[1])?;
                let hkl = parse_hkl_hex(parts[2]);
                return Some(InputMethod {
                    lang_id,
                    profile,
                    profile_type: TF_PROFILETYPE_INPUTPROCESSOR,
                    hkl,
                });
            }
            return None;
        }

        // 旧格式: "0x08040804"（纯 HKL）
        let hkl = parse_hkl_hex(s);
        Some(InputMethod {
            lang_id: (hkl & 0xFFFF) as u16,
            profile: GUID::zeroed(),
            profile_type: 2, // TF_PROFILETYPE_KEYBOARDLAYOUT
            hkl,
        })
    }
}

/// 解析 GUID 字符串，格式: {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}
fn parse_guid(s: &str) -> Option<GUID> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let data1 = u32::from_str_radix(parts[0], 16).ok()?;
    let data2 = u16::from_str_radix(parts[1], 16).ok()?;
    let data3 = u16::from_str_radix(parts[2], 16).ok()?;
    let data4_hex = format!("{}{}", parts[3], parts[4]);
    if data4_hex.len() != 16 {
        return None;
    }
    let mut data4 = [0u8; 8];
    for i in 0..8 {
        data4[i] = u8::from_str_radix(&data4_hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(GUID::from_values(data1, data2, data3, data4))
}

/// 解析 HKL 十六进制字符串
fn parse_hkl_hex(s: &str) -> usize {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        s.parse::<usize>().unwrap_or(0)
    }
}

// ─── TSF 管理器（全局单例）───────────────────────────────────────────

/// 全局 TSF Profile Manager（仅主线程访问，COM STA 模型）
static mut PROFILE_MGR: Option<ITfInputProcessorProfileMgr> = None;

/// 初始化 COM 和 TSF Profile Manager
pub fn init_tsf() -> Result<(), String> {
    unsafe {
        // 初始化 COM（多次调用不会出错）
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        // 如果已经初始化则跳过
        if (*std::ptr::addr_of_mut!(PROFILE_MGR)).is_some() {
            return Ok(());
        }

        // 创建 ITfInputProcessorProfileMgr 实例
        let mgr: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("创建 ITfInputProcessorProfileMgr 失败: {}", e))?;

        *std::ptr::addr_of_mut!(PROFILE_MGR) = Some(mgr);
        log::info!("TSF 初始化成功");
    }
    Ok(())
}

/// 获取全局 TSF 管理器引用
unsafe fn get_profile_mgr() -> Option<&'static ITfInputProcessorProfileMgr> {
    unsafe { (*std::ptr::addr_of_mut!(PROFILE_MGR)).as_ref() }
}

/// 获取前台窗口的输入法状态
pub fn get_foreground_input_method() -> Option<InputMethod> {
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
        get_thread_input_method(thread_id)
    }
}

/// 获取指定线程的输入法状态
fn get_thread_input_method(thread_id: u32) -> Option<InputMethod> {
    // 尝试 TSF
    if let Some(mgr) = unsafe { get_profile_mgr() } {
        if let Some(im) = get_tsf_profile(mgr) {
            return Some(im);
        }
    }

    // 兜底：GetKeyboardLayout
    let hkl = unsafe { GetKeyboardLayout(thread_id) };
    let hkl_val = hkl.0 as usize;
    Some(InputMethod {
        lang_id: (hkl_val & 0xFFFF) as u16,
        profile: GUID::zeroed(),
        profile_type: 2,
        hkl: hkl_val,
    })
}

/// 通过 TSF 获取当前线程的活动 Profile
fn get_tsf_profile(mgr: &ITfInputProcessorProfileMgr) -> Option<InputMethod> {
    unsafe {
        let mut profile: TF_INPUTPROCESSORPROFILE = std::mem::zeroed();

        mgr.GetActiveProfile(&GUID_TFCAT_TIP_KEYBOARD, &mut profile).ok()?;

        let hkl_val = profile.hkl.0 as usize;

        log::debug!(
            "TSF Profile: lang={:04X}, type={}, clsid={:08X}, guid={:08X}-{:04X}-{:04X}, hkl=0x{:08X}",
            profile.langid,
            profile.dwProfileType,
            profile.clsid.data1,
            profile.guidProfile.data1,
            profile.guidProfile.data2,
            profile.guidProfile.data3,
            hkl_val,
        );

        Some(InputMethod {
            lang_id: profile.langid,
            profile: profile.guidProfile,
            profile_type: profile.dwProfileType,
            hkl: hkl_val,
        })
    }
}

/// 激活指定的输入法
pub fn activate_input_method(im: &InputMethod) -> bool {
    // 尝试 TSF
    if let Some(mgr) = unsafe { get_profile_mgr() } {
        if activate_tsf_profile(mgr, im) {
            return true;
        }
    }

    // 兜底：ActivateKeyboardLayout
    unsafe {
        let hkl = windows::Win32::UI::Input::KeyboardAndMouse::HKL(im.hkl as *mut _);
        ActivateKeyboardLayout(hkl, KLF_SETFORPROCESS).is_ok()
    }
}

/// 通过 TSF 激活指定 Profile
fn activate_tsf_profile(mgr: &ITfInputProcessorProfileMgr, im: &InputMethod) -> bool {
    // 如果 Profile GUID 为零（兜底数据），无法使用 TSF
    if im.profile == GUID::zeroed() {
        return false;
    }

    unsafe {
        let result = mgr.ActivateProfile(
            im.profile_type,
            im.lang_id,
            &GUID::zeroed(),  // clsid
            &im.profile,
            windows::Win32::UI::Input::KeyboardAndMouse::HKL::default(),
            0,  // dwflags
        );

        if result.is_ok() {
            log::debug!(
                "TSF ActivateProfile 成功: lang={:04X}, guid={:08X}-{:04X}-{:04X}",
                im.lang_id,
                im.profile.data1,
                im.profile.data2,
                im.profile.data3,
            );
            true
        } else {
            log::warn!("TSF ActivateProfile 失败: {:?}", result);
            false
        }
    }
}
