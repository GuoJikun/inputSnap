use std::sync::Arc;
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject,
    GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, RGBQUAD, HDC,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, CreateIconIndirect,
    DefWindowProcW, DestroyIcon, GetCursorPos, ICONINFO,
    PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenu, TRACK_POPUP_MENU_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW,
    WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WM_USER,
    CW_USEDEFAULT, CS_HREDRAW, CS_VREDRAW, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    MF_UNCHECKED, TPM_LEFTALIGN, TPM_LEFTBUTTON,
};

use crate::config::AppState;

const WM_TRAYICON: u32 = WM_USER + 1;
const ID_TRAYICON: u32 = 1;
const WS_OVERLAPPED: WINDOW_STYLE = WINDOW_STYLE(0);
const TPM_RETURNCMD: u32 = 0x0100;

static mut APP_STATE: Option<Arc<AppState>> = None;

pub fn create_tray_icon(state: Arc<AppState>) -> Result<HWND, String> {
    unsafe { APP_STATE = Some(state); }

    let hwnd = create_hidden_window()?;

    unsafe {
        let icon = create_colored_icon();

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAYICON;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = icon;

        let tip: Vec<u16> = "InputSnap - 输入法自动切换\0".encode_utf16().collect();
        let len = tip.len().min(128);
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        let _ = DestroyIcon(icon);
        log::info!("托盘图标创建成功");
    }

    Ok(hwnd)
}

fn create_hidden_window() -> Result<HWND, String> {
    unsafe {
        let class_name: Vec<u16> = "InputSnapTray\0".encode_utf16().collect();

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..std::mem::zeroed()
        };

        RegisterClassW(&wnd_class);

        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(WS_OVERLAPPED.0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            None,
            None,
        )
        .map_err(|e| format!("创建隐藏窗口失败: {}", e))
    }
}

// "IS" 两个字母的 5x7 点阵字模（1=填充，0=背景）
const GLYPH_I: [u8; 7] = [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111];
const GLYPH_S: [u8; 7] = [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110];

unsafe fn create_colored_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
    // 原生 16x16 绘制，与托盘显示尺寸 1:1 对应，避免系统缩放导致模糊
    let (w, h) = (16i32, 16i32);

    let hdc_screen: HDC = GetDC(None);

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..std::mem::zeroed()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let hbitmap = match CreateDIBSection(
        Some(hdc_screen),
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    ) {
        Ok(bitmap) => bitmap,
        Err(_) => {
            let _ = ReleaseDC(None, hdc_screen);
            return windows::Win32::UI::WindowsAndMessaging::HICON::default();
        }
    };

    // 填充紫色背景 (RGB 138, 43, 226 -> 0x00E22A8A 为 0xBBGGRR 字节序)，四角半径 2 的圆角
    if !bits.is_null() {
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
        const RADIUS: i32 = 2;
        const BG_COLOR: u32 = 0xFFE22A8A; // 0xAABBGGRR：Alpha=FF, B=E2, G=2A, R=8A => 紫色

        for y in 0..h {
            for x in 0..w {
                // 判断像素是否落在圆角圆弧外（四个角各 2x2 区域）
                let cx = x.min(w - 1 - x);
                let cy = y.min(h - 1 - y);
                let inside = if cx < RADIUS && cy < RADIUS {
                    // 角部：到圆心 (RADIUS, RADIUS) 的距离需在圆弧内
                    let dx = cx - RADIUS;
                    let dy = cy - RADIUS;
                    dx * dx + dy * dy <= RADIUS * RADIUS
                } else {
                    true
                };
                pixels[(y * w + x) as usize] = if inside { BG_COLOR } else { 0x00000000 };
            }
        }

        // 两个字母水平并列（5x7 点阵原尺寸），间距 1px，总宽 11px 居中
        let gap = 1i32;
        let total_w = 5 * 2 + gap; // 11
        let start_x = (w - total_w) / 2 + 1; // (16-11)/2 + 1 = 3
        let start_y = (h - 7) / 2 + 1; // (16-7)/2 + 1 = 5

        for (glyph_idx, glyph) in [GLYPH_I, GLYPH_S].iter().enumerate() {
            let base_x = start_x + (glyph_idx as i32) * (5 + gap);
            for row in 0..7i32 {
                for col in 0..5i32 {
                    if (glyph[row as usize] >> (4 - col)) & 1 == 1 {
                        let px = base_x + col;
                        let py = start_y + row;
                        pixels[(py * w + px) as usize] = 0xFFFFFFFF; // 白色
                    }
                }
            }
        }
    }

    let hbm_mask = CreateBitmap(w, h, 1, 1, None);

    let icon = match CreateIconIndirect(&ICONINFO {
        fIcon: BOOL::from(true),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbitmap,
    }) {
        Ok(icon) => icon,
        Err(_) => windows::Win32::UI::WindowsAndMessaging::HICON::default(),
    };

    let _ = DeleteObject(hbitmap.into());
    let _ = DeleteObject(hbm_mask.into());
    let _ = ReleaseDC(None, hdc_screen);

    icon
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    match msg {
        WM_TRAYICON => {
            let mouse_msg = (lparam.0 & 0xFFFF) as u32;
            match mouse_msg {
                WM_LBUTTONUP => {
                    log::info!("托盘图标左键点击");
                    if let Some(ref state) = APP_STATE {
                        let new_enabled = state.toggle_enabled();
                        log::info!("状态切换为: {}", if new_enabled { "启用" } else { "暂停" });
                    }
                }
                WM_RBUTTONUP => {
                    log::info!("托盘图标右键点击");
                    show_tray_menu(hwnd);
                }
                _ => {}
            }
            windows::Win32::Foundation::LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            windows::Win32::Foundation::LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let hmenu = CreatePopupMenu().unwrap_or_default();

    let status = if let Some(ref state) = APP_STATE {
        if state.is_enabled() { "状态: 运行中\0" } else { "状态: 已暂停\0" }
    } else {
        "状态: 运行中\0"
    };
    let status_w: Vec<u16> = status.encode_utf16().collect();
    let _ = AppendMenuW(hmenu, MF_GRAYED, 0, PCWSTR(status_w.as_ptr()));

    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR(std::ptr::null()));

    let toggle = if let Some(ref state) = APP_STATE {
        if state.is_enabled() { "暂停自动切换\0" } else { "恢复自动切换\0" }
    } else {
        "暂停自动切换\0"
    };
    let toggle_w: Vec<u16> = toggle.encode_utf16().collect();
    let _ = AppendMenuW(hmenu, MF_STRING, 2, PCWSTR(toggle_w.as_ptr()));

    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR(std::ptr::null()));

    // 开机自启选项（根据注册表状态显示勾选标记）
    let auto_start_on = crate::registry::is_auto_start_enabled();
    let auto_start_text = "开机自启\0";
    let auto_start_flags = MF_STRING | if auto_start_on { MF_CHECKED } else { MF_UNCHECKED };
    let auto_start_w: Vec<u16> = auto_start_text.encode_utf16().collect();
    let _ = AppendMenuW(hmenu, auto_start_flags, 4, PCWSTR(auto_start_w.as_ptr()));

    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR(std::ptr::null()));

    let quit_w: Vec<u16> = "退出\0".encode_utf16().collect();
    let _ = AppendMenuW(hmenu, MF_STRING, 3, PCWSTR(quit_w.as_ptr()));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);

    let cmd = TrackPopupMenu(
        hmenu,
        TRACK_POPUP_MENU_FLAGS(TPM_LEFTALIGN.0 | TPM_LEFTBUTTON.0 | TPM_RETURNCMD),
        pt.x,
        pt.y,
        Some(0),
        hwnd,
        None,
    );

    match cmd.0 as usize {
        2 => {
            if let Some(ref state) = APP_STATE {
                let new_enabled = state.toggle_enabled();
                log::info!("状态切换为: {}", if new_enabled { "启用" } else { "暂停" });
            }
        }
        3 => {
            log::info!("用户请求退出");
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
        }
        4 => {
            // 用户手动切换开机自启，标记为已配置
            crate::registry::mark_auto_start_configured();
            if crate::registry::is_auto_start_enabled() {
                match crate::registry::disable_auto_start() {
                    Ok(()) => log::info!("已禁用开机自启"),
                    Err(e) => log::error!("禁用开机自启失败: {}", e),
                }
            } else {
                match crate::registry::enable_auto_start() {
                    Ok(()) => log::info!("已启用开机自启"),
                    Err(e) => log::error!("启用开机自启失败: {}", e),
                }
            }
        }
        _ => {}
    }
}

fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAYICON;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        log::info!("托盘图标已移除");
    }
}
