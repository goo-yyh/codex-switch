---
title: 国内模型
description: Codex Switch 国内模型与使用说明。
---

预设参考了 CC Switch 固定版本，候选模型由本项目维护，只作为新建配置的初始值；不覆盖已保存的用户地址、协议或模型，也不代表账号拥有对应权限。

| 服务     | 默认模型            | API 地址                                            | 接口      |
| -------- | ------------------- | --------------------------------------------------- | --------- |
| 千问     | `qwen3.8-max`       | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Responses |
| MiniMax  | `MiniMax-M3`        | `https://api.minimaxi.com/v1`                       | Responses |
| 智谱 GLM | `glm-5.3`           | `https://open.bigmodel.cn/api/v1`                   | Responses |
| Kimi     | `kimi-k3`           | `https://api.moonshot.cn/v1`                        | Responses |
| DeepSeek | `deepseek-v4-flash` | `https://api.deepseek.com`                          | Responses |

「服务套餐」另提供 Kimi Coding 与千问 Token Plan。选择套餐会载入对应地址、模型和能力，并清空待填写的密钥。所有服务的 API 地址均可自行修改。

上下文长度、思考档位和图像支持按模型预设载入，选中模型后，点击该模型行后的「编辑」修改；确认修改后还需保存配置，取消则丢弃本次编辑。未知型号采用 CC Switch 的模板和能力注册表；请按实际服务能力调整。

## 一个配置，多个模型

新增配置时填写一个 API Key，并勾选 1–20 个模型。点击「保存配置」以一个具名配置保存并回到首页，不会自动发送测试请求。先关闭 Codex Switch 后，编辑时可以增删该配置的模型，地址与完整 URL 模式未变化时，留空 Key 即保留原密钥。

首页可同时勾选多个厂商的配置并应用到 Codex，模型显示为 `配置名称-模型`。模型旁的「已配置」表示相同厂商、API 地址与接口下已有配置使用它；本次勾选只编辑当前配置，不会删除其他配置的模型。

列表提供常用模型，也支持输入其他模型 ID。它不是账号授权清单；套餐和模型权限以服务商为准。

## 按量 API 与 Coding Plan

两类产品可能使用不同密钥、地址和可用模型。密钥格式相似不代表能够互用。出现 401、403 或 404 时，先在供应商控制台核对套餐，不要把同一个密钥发往不明第三方端点。

## 官方入口

- [千问文档](https://help.aliyun.com/zh/model-studio/)
- [MiniMax 文档](https://platform.minimaxi.com/docs/api-reference/text-openai-api)
- [智谱文档](https://docs.bigmodel.cn/)
- [Kimi 文档](https://platform.moonshot.cn/docs/)
- [DeepSeek 文档](https://api-docs.deepseek.com/)

## 验证代表什么

点击「测试配置」后，应用逐个检查所选模型的基础文本响应，失败时显示对应模型。测试不会保存或启用配置，保存也不依赖测试通过。工具调用、多轮上下文、流式输出通过独立协议测试覆盖；特定供应商的实测结果应以测试回执为准，不能由连通状态推导完整 App 兼容性。

详细字段和继承关系见[配置字段与模型能力](/docs/configuration/)，远程压缩偏好见[通用设置](/docs/settings/)。
