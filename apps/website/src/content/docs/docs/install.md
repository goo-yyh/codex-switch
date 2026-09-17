---
title: 安装与源码构建
description: 了解 Codex Switch 的 macOS 与 Windows 构建依赖、安装步骤、更新和卸载方式，以及当前公开安装包的发布状态。
---

当前版本提供源代码构建，公开安装包尚未发布。下载入口以[下载页](/download/)为准。

## 准备

- 安装官方 [Codex App](https://chatgpt.com/download/)。
- 准备模型服务商提供的 API Key 和可用额度。
- 使用源代码构建时，需要 Node.js 22.12+、pnpm 10、Rust stable，以及平台编译工具。

## 从源码构建

```sh
git clone https://github.com/goo-yyh/codex-switch.git
cd codex-switch
pnpm install --frozen-lockfile
pnpm --filter @codex-switch/desktop tauri build
```

macOS 需要 Xcode Command Line Tools。构建产物位于 `target/release/bundle/`，包含 `.app` 和 `.dmg`。当前命令为本机构架生成安装包；Intel 与 Apple Silicon 需要对应构建环境或显式目标。

本地预览构建未经 Apple 公证。正式分发需要维护者完成签名、公证和发布验证；不要通过关闭系统整体安全设置来安装。

## Windows

需要 Microsoft C++ Build Tools 与 WebView2。使用以下命令生成 NSIS 安装程序：

```sh
pnpm --filter @codex-switch/desktop tauri build --config src-tauri/tauri.windows.conf.json
```

Windows 构建由 CI 提供检查入口，当前本地 macOS 验证不能证明 Windows 实机可用。

## 更新与卸载

第一版不自动更新。升级前正常退出 Codex，再完全退出 Codex Switch；安装新版即可保留本地连接元数据。

卸载前请先关闭总开关，确认原配置恢复，然后正常退出 Codex 与 Codex Switch。仅删除应用不会自动执行恢复。系统凭据库与应用数据可能仍然保留，应在卸载前通过应用删除不再需要的连接。
