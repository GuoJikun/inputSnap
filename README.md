# InputSnap 输入法自动切换

InputSnap 是一款 Windows 输入法自动切换工具：记录每个应用程序窗口的输入法状态，切换窗口时自动恢复对应的输入法，无需手动切换中英文。

## 功能特性

- **窗口级记忆**：按进程记录每个应用窗口的键盘布局（输入法）状态
- **自动恢复**：切换窗口时自动激活该窗口上次使用的输入法
- **系统托盘**：托盘图标常驻，右键菜单可查看/退出程序
- **日志记录**：运行日志自动轮转，便于排查问题
- **低开销**：基于 WinEventHook 监听前台窗口变化，内存占用小

## 系统要求

- Windows 10 1809（Build 18362）及以上
- 以**管理员权限**运行（程序内部使用 Win32 API 需要）

## 构建

```bash
# 开发构建（日志输出到控制台 + 文件）
cargo build

# 发布构建（仅文件日志，无控制台窗口）
cargo build --release

# 运行测试
cargo test
```

发布版 exe 位于 `target/release/input_snap.exe`。

## 打包 MSIX 与安装

使用 `build.ps1` 一键构建并打包：

```powershell
# 仅编译 exe
.\build.ps1 -SkipMsix

# 完整构建并打包 MSIX（使用 devcert.pfx 自签名）
.\build.ps1

# 打包并以开发模式安装到本机（需开启开发者模式，支持未签名包）
.\build.ps1 -Install
```

> 说明：MSIX 格式强制要求签名才能安装。
> - 本地自测：用 `devcert.pfx` 签名（`build.ps1` 默认路径 `$HOME\devcert.pfx`）
> - 发布到 Microsoft Store：由微软自动签名，打包时不需签名

### 发布到 Microsoft Store

1. 推送 `v*` 前缀的 git 标签触发 CI，自动构建并打包**未签名** MSIX
2. 在 GitHub Actions 页面下载 `InputSnap-msix` 构建产物
3. 登录 [Partner Center](https://partner.microsoft.com/dashboard) 手动上传提交

发布时需保证 MSIX 清单中的 `Publisher` 与 Partner Center 账户一致（`CN=GuoJikun`）。

## 日志

- 位置：`<安装目录>/logs/InputSnap.log`
- 单文件最大 1 MB，最多保留 5 个文件

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

## 技术栈

- Rust（edition 2021）
- [windows crate](https://crates.io/crates/windows) 0.62（Windows API）
- [tray-icon](https://crates.io/crates/tray-icon) + [muda](https://crates.io/crates/muda)（系统托盘）
- [log](https://crates.io/crates/log) + [fern](https://crates.io/crates/fern) + [chrono](https://crates.io/crates/chrono)（日志）