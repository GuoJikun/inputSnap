# AGENTS.md

## 语言要求

- **所有输出必须使用中文**
- 代码注释使用中文
- 提交信息使用中文
- 日志输出使用中文

## 项目概述

这是一个 Windows 输入法自动切换工具，记录每个应用程序窗口的输入法状态，切换窗口时自动恢复对应的输入法。

## 技术栈

- 语言：Rust
- Windows API：`windows` crate (0.61)
- 系统托盘：`tray-icon` (0.19) + `muda` (0.15)
- 日志：`log` + `fern` + `chrono`

## 项目结构

```
src/
├── main.rs          # 入口：日志初始化、管理员检查
├── ime.rs           # 输入法核心 FFI 封装
├── watcher.rs       # WinEventHook 监听前台窗口
├── registry.rs      # 注册表读写
├── tray.rs          # 系统托盘 + 菜单
├── config.rs        # 运行时状态
└── log_writer.rs    # 日志轮转写入器
```

## 构建命令

```bash
# 开发构建
cargo check

# 发布构建
cargo build --release

# 运行测试
cargo test
```

## 核心 API

- `GetKeyboardLayout()` - 获取键盘布局
- `ActivateKeyboardLayout()` - 激活键盘布局
- `SetWinEventHook()` - 监听窗口事件
- `GetProcessImageFileNameW()` - 获取进程名

## 注册表位置

```
HKEY_CURRENT_USER\Software\KeepAppInputStatus
```

## 日志配置

- 文件名：`InputSnap.log`
- 目录：`<安装目录>/logs/`
- 单文件最大：1MB
- 最多保留：5个文件

## 开发注意事项

1. 必须以管理员权限运行
2. 所有 Windows API 调用都是 unsafe 的
3. 回调函数中不能进行复杂操作
4. 使用 `log` 宏输出日志，不要使用 `println!`
