---
title: CC Switch 开源致谢与代码来源
description: 了解 Codex Switch 参考和复用的 CC Switch 协议转换、流处理与模型目录代码，查看固定上游版本、MIT 许可与维护范围。
---

Codex Switch 参考并直接复用了 [CC Switch](https://github.com/farion1231/cc-switch) 的部分开源代码和测试。感谢 CC Switch 作者 Jason Young 及社区贡献者。

## 复用了哪些代码

- Chat / Responses 请求与响应转换、流式事件处理。
- 工具调用历史、工具 schema 与相关辅助逻辑和测试。
- SSE、JSON / 媒体辅助逻辑。
- 模型能力注册表、模型目录构建逻辑及三个模型模板。
- 部分服务商初始预设；本项目另行维护候选模型和本地适配。

上游固定在提交 [`06082e189d65e6d6dbadc35dacdac1ce6c79d89a`](https://github.com/farion1231/cc-switch/tree/06082e189d65e6d6dbadc35dacdac1ce6c79d89a)。代码保存在本仓库 `crates/cc-switch-codex`，通过文件及提取片段的 SHA-256 清单追踪来源。

## 本项目维护的部分

桌面界面、配置备份与恢复事务、系统凭据管理、HTTP 连接生命周期、所选模型路由、原生应用管理和文档站由 Codex Switch 维护。本地适配增加了有界会话缓存、请求校验等约束。

这是从上游应用中提取的源码模块，不是 CC Switch 官方 SDK。没有整体引入上游数据库或通用路由器；源码中保留的上游熔断器不用于本地网关。当前请求失败不会自动切换服务或协议。

完整复用范围、差异及更新规则见仓库 [UPSTREAM.md](https://github.com/goo-yyh/codex-switch/blob/main/crates/cc-switch-codex/UPSTREAM.md)。

## 许可证与独立性

本项目采用 [MIT 许可证](https://github.com/goo-yyh/codex-switch/blob/main/LICENSE)。复用的 CC Switch 代码保留原始 [MIT 许可证与版权声明](https://github.com/goo-yyh/codex-switch/blob/main/crates/cc-switch-codex/LICENSE)：Copyright (c) 2025 Jason Young。

Codex Switch 是社区独立项目，与 OpenAI 无隶属关系。引用上游预设或代码不等于继承上游测试结论；本项目的验证范围见[兼容范围](/docs/compatibility/)。
