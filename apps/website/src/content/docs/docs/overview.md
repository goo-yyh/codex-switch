---
title: 产品介绍
description: Codex Switch 产品介绍与使用说明。
---

Codex Switch 是一个为 **Codex App** 管理模型连接的独立桌面应用。你可以添加国内模型或coding plan，每个配置包含一个 Key 和多个模型，首页可同时勾选多个配置提供给 Codex。

## 第一版能做什么

- 内置千问、MiniMax、智谱、Kimi、DeepSeek 的国内按量 API 预设。
- 支持自定义 HTTPS 中转站，选择 Responses 或 Chat Completions 接口。
- 自定义配置名称并保存，按需手动测试所选模型；首页多选应用，再打开 Codex App。
- 开启前保存原配置，关闭后恢复开启前的文件。
- 密钥保存于 macOS Keychain 或 Windows 系统凭据库。

## 使用流程

[安装应用](/docs/install/) → [添加配置](/docs/quickstart/) → 勾选并应用 → 打开 Codex。

## 当前阶段

0.1 是开发预览版。代码、离线测试与构建不代表在所有 Codex App 版本上通过实机验证。首次使用前请阅读[兼容范围](/docs/compatibility/)。公开签名安装包尚未发布。

产品只围绕 Codex App 提供入口。Codex 的工具可能共用配置目录，因此同一目录里的其他客户端也可能读取这些配置；第一版不对 CLI 或 IDE 扩展提供适配保证。
