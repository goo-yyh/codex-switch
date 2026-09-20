---
title: 更新日志
description: 查看 Codex Switch 的中英文切换、多模型管理、自动恢复、托盘操作和文档更新。
---

已发布版本和安装包见 [GitHub Releases](https://github.com/goo-yyh/codex-switch/releases)。

## v0.2.0 · 2026 年 9 月 20 日

[发布说明](https://github.com/goo-yyh/codex-switch/releases/tag/v0.2.0) · [下载与安装](/docs/install/)

- **服务操作**：开启后不再显示强制弹窗，底部开关和打开 Codex 按钮保持可用，其余区域由遮罩锁定；重启仍需确认，关闭失败可在操作栏查看错误并重试。
- **模型目录 v2**：内置更新后的 21 个普通候选，新增千问 `qwen3.8-omni-flash` 和智谱 `glm-5.3-flashx`，DeepSeek 普通候选保留两个。已有配置、参数和所选模型保持不变。
- **目录同步**：启动时检查站点模型目录，仅在远程版本更高时合并新增模型；失败时继续使用本地目录。
- **安装包**：提供 macOS Apple Silicon、macOS Intel 和 Windows x64 安装包及 SHA-256 校验文件，继续采用未签名发布方式。

## v0.1.0 · 2026 年 9 月 18 日

[发布说明](https://github.com/goo-yyh/codex-switch/releases/tag/v0.1.0) · [下载与安装](/docs/install/)

- **公开安装包**：提供 macOS Apple Silicon、macOS Intel 和 Windows x64 安装包及 SHA-256 校验文件。安装包未签名，macOS 应用未经公证。
- **更新提示**：后台检查正式版本，提供下载入口，安装由用户手动完成。

- **中英文切换**：应用界面和托盘同步切换，重新打开保留语言选择。
- **模型配置**：支持多模型、多配置和套餐预设；模型列表直接展开，可分别调整能力。
- **服务操作**：开启时锁定设置，关闭后自动恢复原配置；保存或选择配置不会自动开启。
- **托盘操作**：支持连续多选，简化为选择配置、服务开关、打开面板和退出应用。
- **上下文压缩**：偏好自动保存，默认关闭远程压缩；移除备用队列与单独保存路由设置的入口。
- **使用文档**：中英文截图教程，合并重复说明，优先展示连接和恢复步骤。

首次使用请看[三步连接](/docs/quickstart/)，下载状态见[安装与下载](/docs/install/)。
