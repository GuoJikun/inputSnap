// 关于窗口：显示程序版本号和升级地址（GitHub / Gitee）
// 链接使用 Win32 原生 SysLink 控件，点击直接打开默认浏览器

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, GetSysColorBrush, SetBkMode,
    COLOR_WINDOW, FONT_CHARSET, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FONT_QUALITY,
    HDC, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_LINK_CLASS, INITCOMMONCONTROLSEX, NMLINK, NMHDR, NM_CLICK,
    NM_RETURN,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, GetSystemMetrics, RegisterClassW, SendMessageW, SetForegroundWindow,
    ShowWindow, TranslateMessage, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, SW_SHOWNORMAL,
    WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WNDCLASS_STYLES, WM_CLOSE, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_NOTIFY, WM_SETFONT, WS_CAPTION, WS_CHILD, WS_POPUP, WS_SYSMENU, WS_TABSTOP,
    WS_VISIBLE, MSG,
};

/// 升级地址（指向 Release 下载页）
const GITHUB_URL: &str = "https://github.com/GuoJikun/inputSnap/releases";
const GITEE_URL: &str = "https://gitee.com/guojikun/inputSnap/releases";

/// 关于窗口是否已关闭（模态循环的退出条件）
static ABOUT_CLOSED: AtomicBool = AtomicBool::new(true);

/// 自定义字体句柄（微软雅黑），窗口销毁时释放
static FONT_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// 弹出"关于"窗口，以 owner 为属主模态显示
pub fn show_about(owner: HWND) {
    unsafe {
        // SysLink 属于公共控件，使用前需要注册控件类
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LINK_CLASS,
        };
        let _ = InitCommonControlsEx(&icc);

        let class_name = wide("InputSnapAbout");
        let wnd_class = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(about_proc),
            hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
            // 使用系统窗口背景色画刷，与控件透明背景保持一致
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wnd_class);

        // 窗口在屏幕中央显示
        let (ww, wh) = (360i32, 180i32);
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);

        let title = wide("关于");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            (sx - ww) / 2,
            (sy - wh) / 2,
            ww,
            wh,
            Some(owner),
            None,
            None,
            None,
        )
        .unwrap_or_default();

        if hwnd.0.is_null() {
            log::error!("创建关于窗口失败");
            return;
        }

        create_controls(hwnd);

        // 模态显示：禁用属主窗口，进入独立消息循环直到关于窗口关闭
        ABOUT_CLOSED.store(false, Ordering::SeqCst);
        let _ = EnableWindow(owner, false);
        let _ = ShowWindow(hwnd, SW_SHOW);

        let mut msg = MSG::default();
        while !ABOUT_CLOSED.load(Ordering::SeqCst) {
            if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&mut msg);
        }

        let _ = EnableWindow(owner, true);
        let _ = SetForegroundWindow(owner);
    }
}

/// 创建窗口内的静态文本和可点击链接控件
unsafe fn create_controls(parent: HWND) { unsafe {
    let static_class = wide("STATIC");
    let link_class = wide("SysLink");
    // 创建微软雅黑字体（-14 约 10.5pt@96DPI）
    let font_name = wide("微软雅黑");
    let font = CreateFontW(
        -14, 0, 0, 0,
        400,   // FW_NORMAL
        0, 0, 0,
        FONT_CHARSET(1),           // DEFAULT_CHARSET
        FONT_OUTPUT_PRECISION(0),  // OUT_DEFAULT_PRECIS
        FONT_CLIP_PRECISION(0),    // CLIP_DEFAULT_PRECIS
        FONT_QUALITY(0),           // DEFAULT_QUALITY
        0,
        PCWSTR(font_name.as_ptr()),
    );
    FONT_HANDLE.store(font.0 as isize, Ordering::SeqCst);

    let mut y = 16i32;
    let name = create_child(
        parent,
        &static_class,
        "InputSnap",
        WINDOW_STYLE(0),
        16,
        y,
        320,
        20,
    );
    y += 26;
    let ver = create_child(
        parent,
        &static_class,
        &format!("版本: v{}", env!("CARGO_PKG_VERSION")),
        WINDOW_STYLE(0),
        16,
        y,
        320,
        20,
    );
    y += 32;
    let hint = create_child(
        parent,
        &static_class,
        "下载地址:",
        WINDOW_STYLE(0),
        16,
        y,
        70,
        20,
    );
    let link = create_child(
        parent,
        &link_class,
        &format!(
            r#"<a href="{}">GitHub</a>    <a href="{}">Gitee</a>"#,
            GITHUB_URL, GITEE_URL
        ),
        WS_TABSTOP,
        86,
        y,
        240,
        20,
    );

    // 统一设置为系统默认 GUI 字体
    for control in [name, ver, hint, link] {
        let _ = SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}}

/// 创建子控件的通用封装
unsafe fn create_child(
    parent: HWND,
    class: &[u16],
    text: &str,
    extra_style: WINDOW_STYLE,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> HWND { unsafe {
    let text_w = wide(text);
    match CreateWindowExW(
        WINDOW_EX_STYLE(0),
        PCWSTR(class.as_ptr()),
        PCWSTR(text_w.as_ptr()),
        WS_CHILD | WS_VISIBLE | extra_style,
        x,
        y,
        w,
        h,
        Some(parent),
        None,
        None,
        None,
    ) {
        Ok(hwnd) => hwnd,
        Err(e) => {
            log::error!("创建子控件失败: {}", e);
            HWND(std::ptr::null_mut())
        }
    }
}}

unsafe extern "system" fn about_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT { unsafe {
    match msg {
        WM_CTLCOLORSTATIC => {
            // 文本背景模式设为透明，并返回与窗口底色一致的画刷。
            // 注意：SysLink 控件不支持 NULL_BRUSH 透明绘制（返回 NULL_BRUSH
            // 会导致链接完全不渲染），必须返回真实画刷
            let hdc = HDC(wparam.0 as *mut _);
            let _ = SetBkMode(hdc, TRANSPARENT);
            LRESULT(GetSysColorBrush(COLOR_WINDOW).0 as isize)
        }
        WM_NOTIFY => {
            // SysLink 点击事件：取出链接 URL 并打开浏览器
            let nmhdr = &*(lparam.0 as *const NMHDR);
            if nmhdr.code == NM_CLICK || nmhdr.code == NM_RETURN {
                let nmlink = &*(lparam.0 as *const NMLINK);
                let url = wide_slice_to_string(&nmlink.item.szUrl);
                open_url(&url);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            // 释放自定义字体资源
            let font_raw = FONT_HANDLE.swap(0, Ordering::SeqCst);
            if font_raw != 0 {
                let hfont = windows::Win32::Graphics::Gdi::HFONT(font_raw as *mut _);
                let _ = DeleteObject(hfont.into());
            }
            ABOUT_CLOSED.store(true, Ordering::SeqCst);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}}

/// 调用系统默认浏览器打开链接
unsafe fn open_url(url: &str) { unsafe {
    if url.is_empty() {
        return;
    }
    let url_w = wide(url);
    let op = wide("open");
    let result = ShellExecuteW(
        None,
        PCWSTR(op.as_ptr()),
        PCWSTR(url_w.as_ptr()),
        PCWSTR(std::ptr::null()),
        PCWSTR(std::ptr::null()),
        SW_SHOWNORMAL,
    );
    // ShellExecuteW 返回值大于 32 表示成功
    if result.0 as usize <= 32 {
        log::warn!("打开链接失败: {}", url);
    } else {
        log::info!("已打开链接: {}", url);
    }
}}

/// 字符串转以 NUL 结尾的 UTF-16 序列
fn wide(s: &str) -> Vec<u16> {
    format!("{}\0", s).encode_utf16().collect()
}

/// 将以 NUL 结尾的 UTF-16 数组转为 String
fn wide_slice_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
