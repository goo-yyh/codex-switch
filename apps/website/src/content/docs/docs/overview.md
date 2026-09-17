---
title: 产品介绍
description: 用 Codex Switch 管理多个模型服务，了解使用流程、配置恢复和开源来源。
---

Codex Switch 是为 **Codex App** 管理模型连接的独立桌面应用。一个配置包含一个 Key 和多个模型，首页可同时选择多个配置；开启前备份原配置，关闭后恢复。

[![Codex Switch 实际界面：Kimi 与 DeepSeek 多配置启用](/screenshots/enabled.png)](/screenshots/enabled.png)

_当前应用浏览器预览截图，使用示例配置；连接和备份状态为模拟数据。_

## 从这里开始

[安装与下载](/docs/install/) → [三步连接（截图教程）](/docs/quickstart/) → [配置字段与模型能力](/docs/configuration/)。

## 可以做什么

- 连接千问、MiniMax、智谱 GLM、Kimi、DeepSeek，或[套餐与自定义 HTTPS 接口](/docs/relay/)。
- 保存多个配置，每个配置选择 1–20 个模型；按模型覆盖地址、接口与能力。
- 按需测试配置，开启后在 Codex 模型列表中选择 `配置名称-模型`。
- 使用系统凭据库保存 Key，在本机转发请求，不经过 Codex Switch 云端。
- 通过托盘管理后台连接，关闭总开关[恢复原配置](/docs/switch/)。

## 当前阶段

支持 macOS 与 Windows，当前为 0.1.0 开发预览版，公开签名安装包尚未发布。离线测试、构建和供应商接口检查不等于完整 Codex App 实机验证，请先阅读[兼容范围](/docs/compatibility/)。

Codex 工具可能共用配置目录，同一目录里的其他客户端也可能读取这些配置；当前不对 CLI 或 IDE 扩展提供适配保证。

## 开源来源

本项目参考并直接复用 [CC Switch](https://github.com/farion1231/cc-switch) 的部分协议转换、流处理、工具历史与模型目录代码，保留上游 MIT 版权声明。具体来源、固定提交及本地维护范围见[开源致谢](/docs/open-source/)。
