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
    CW_USEDEFAULT, CS_HREDRAW, CS_VREDRAW, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    TPM_LEFTALIGN, TPM_LEFTBUTTON,
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
            hInstance: GetModuleHandleW(None).unwrap().into(),
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

unsafe fn create_colored_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
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
    let hbitmap = CreateDIBSection(
        Some(hdc_screen),
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        None,
        0,
    )
    .unwrap_or_default();

    if !bits.is_null() {
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let idx = (y * w as usize + x) * 4;
                pixels[idx] = (x * 16) as u8;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = (y * 16) as u8;
                pixels[idx + 3] = 255;
            }
        }
    }

    let hbm_mask = CreateBitmap(w, h, 1, 1, None);

    let icon = CreateIconIndirect(&ICONINFO {
        fIcon: BOOL::from(true),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbitmap,
    })
    .unwrap_or_default();

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
