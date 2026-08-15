# InputSnap 输入法自动切换

InputSnap 是一款 Windows 输入法自动切换工具：记录每个应用程序窗口的输入法状态，切换窗口时自动恢复对应的输入法，无需手动切换中英文。

## 功能特性

- **窗口级记忆**：按进程记录每个应用窗口的键盘布局（输入法）状态
- **自动恢复**：切换窗口时自动激活该窗口上次使用的输入法
- **系统托盘**：托盘图标常驻，右键菜单可查看/退出程序
- **日志记录**：运行日志自动轮转，便于排查问题
- **低开销**：基于 WinEventHook 监听前台窗口变化，内存占用小
- **管理员权限**：通过 `app.manifest` 声明 `requireAdministrator`，启动时自动请求提权，无需手动右键以管理员运行

## 系统要求

- Windows 10 1809（Build 18362）及以上
- 需要管理员权限（程序内部使用 Win32 API）

## 构建

```bash
# 开发构建（日志输出到控制台 + 文件）
cargo build

# 发布构建（仅文件日志，无控制台窗口）
cargo build --release

# 运行测试
cargo test
```

发布版 exe 位于 `target/release/input_snap.exe`，可直接运行或分发。

### 构建产物说明

- 通过 `build.rs`（winres）嵌入版本信息（1.0.0.0）、图标（`assets/icon.ico`）和 `app.manifest`
- `app.manifest` 声明管理员权限及 Windows 10/11 兼容性
- `main.rs` 顶部 `#![windows_subsystem = "windows"]`，发布版不弹出控制台窗口

## 发布（CI）

推送 `v*` 前缀的 git 标签触发 GitHub Actions：

1. 在 `windows-latest` 上执行 `cargo build --release`
2. 将 `target/release/input_snap.exe` 上传为 `input_snap-exe` 构建产物
3. 到 Actions 页面下载 exe 分发

## 日志

- 位置：`<安装目录>/logs/InputSnap.log`
- 单文件最大 1 MB，最多保留 5 个文件
- 开发构建（debug）同时输出到控制台和文件；发布构建（release）仅写文件日志

## 配置存储

注册表位置：`HKEY_CURRENT_USER\Software\KeepAppInputStatus`

## 项目结构

```
src/
├── main.rs          # 入口：日志初始化、管理员检查、消息循环
├── ime.rs           # 输入法核心 FFI 封装
├── watcher.rs       # WinEventHook 监听前台窗口
├── registry.rs      # 注册表读写
├── tray.rs          # 系统托盘 + 菜单
├── config.rs        # 运行时状态
└── log_writer.rs    # 日志轮转写入器
```

其他：

- `build.rs`          # 编译时嵌入版本信息、图标、清单
- `app.manifest`      # 管理员权限及兼容性声明
- `assets/icon.ico`   # 程序图标

## 技术栈

- Rust（edition 2021）
- [windows crate](https://crates.io/crates/windows) 0.62（Windows API）
- [tray-icon](https://crates.io/crates/tray-icon) + [muda](https://crates.io/crates/muda)（系统托盘）
- [log](https://crates.io/crates/log) + [fern](https://crates.io/crates/fern) + [chrono](https://crates.io/crates/chrono)（日志）