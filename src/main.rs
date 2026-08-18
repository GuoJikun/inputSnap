// 声明为 Windows 窗口子系统，避免启动时弹出控制台窗口
#![windows_subsystem = "windows"]

mod about;
mod config;
mod crash;
mod ime;
mod log_writer;
mod registry;
mod tray;
mod watcher;

use std::sync::Arc;
use config::AppState;

/// 初始化日志系统（同时输出到控制台和文件，支持日志轮转）
fn setup_log() -> Result<(), Box<dyn std::error::Error>> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .ok_or("无法获取可执行文件目录")?
        .to_path_buf();

    let logs_dir = exe_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    let log_writer = log_writer::RotatingWriter::new(
        logs_dir,
        "InputSnap.log",
        1024 * 1024,
        5,
    );

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(Box::new(log_writer) as Box<dyn std::io::Write + Send>);

    // 开发模式下同时输出到控制台，正式运行仅写文件日志
    if cfg!(debug_assertions) {
        dispatch = dispatch.chain(std::io::stdout());
    }

    dispatch.apply()?;

    Ok(())
}

/// 检查是否以管理员权限运行
fn is_admin() -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE(std::ptr::null_mut());
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        let _ = CloseHandle(token);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// 运行 Win32 消息循环
fn run_message_loop() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    log::info!("进入消息循环");

    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    log::info!("消息循环退出");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_log()?;

    // 日志初始化完成后立即安装崩溃捕获，确保 panic/原生异常能写入日志
    crash::install();

    log::info!("========================================");
    log::info!("InputSnap 已启动");
    log::info!("========================================");

    if !is_admin() {
        log::warn!("未以管理员权限运行，部分功能可能无法正常工作");
    } else {
        log::info!("已获得管理员权限");
    }

    // 首次运行时默认启用开机自启（仅在用户从未手动配置过时生效）
    if !registry::is_auto_start_configured() {
        match registry::enable_auto_start() {
            Ok(()) => {
                registry::mark_auto_start_configured();
                log::info!("首次运行，已默认启用开机自启");
            }
            Err(e) => log::warn!("默认启用开机自启失败: {}", e),
        }
    }

    let state = Arc::new(AppState::new());
    log::info!("状态对象创建完成");

    watcher::set_state_ptr(state.clone());
    log::info!("状态指针设置完成");

    log::info!("准备启动窗口监听...");
    let _hook = match watcher::start_watching(state.clone()) {
        Ok(hook) => {
            log::info!("窗口监听启动完成");
            hook
        }
        Err(e) => {
            log::error!("窗口监听启动失败: {}", e);
            return Err(e.into());
        }
    };

    log::info!("准备创建托盘图标...");
    let _hwnd = tray::create_tray_icon(state.clone())?;
    log::info!("托盘图标创建完成");

    log::info!("系统运行中，右键托盘图标可操作");

    // 运行 Win32 消息循环
    run_message_loop();

    log::info!("程序退出");
    Ok(())
}
