# 模型上下文预设核对（2026-09-17）

范围：5 家厂商的 36 个普通候选和 9 个套餐候选。只查阅官方公开文档，不读取密钥、不调用推理接口，也不启动或修改用户正在使用的 Codex。本记录验证的是官方公布的容量，不是长上下文压力测试结果。

## 计算与生效规则

- 预设使用 `floor(容量基数 × 0.8)`，通过整数运算 `容量基数 × 4 / 5` 向下取整，只计算一次。
- 每个候选都显式填写 `modelOverrides[model].contextWindow`，避免小窗口模型继承厂商旗舰模型的容量。厂商和套餐的外层默认值与其默认模型一致。
- 使用上下文总容量，不混用最大输入、最大输出或思考预算。千问原来的 `983616` 是思考模式最大输入长度；其 1,000,000 上下文对应本次预设 800,000。
- K/M 的含义按厂商数值示例核对，不跨厂商统一乘 1024。智谱 1M 文档存在 1,000,000 与 1,048,576 两种示例，本次保守采用较小的 1,000,000；不是声称精确硬上限只有此值。智谱 200K 按官方 Codex 示例的 204,800 换算；Kimi 的 256K/1M 按官方模型配置与 Codex 示例的 262,144/1,048,576 换算。
- 新增配置或重新选择套餐时载入新预设。已有配置的上下文值、用户手填模型与自定义值保留，不静默迁移，也不在保存或启用时再次乘 80%。
- 缩减后的值写入 Codex 模型目录的 `context_window` 和 `max_context_window`。没有设置独立的自动压缩阈值，客户端仍执行自身压缩规则；预留 20% 可降低超限风险，不保证任意大输入、工具结果或输出预算都不会超限。

## 智谱 GLM

官方依据：[模型概览](https://docs.bigmodel.cn/cn/guide/start/model-overview)、[Codex 接入](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)、[套餐模型切换](https://docs.bigmodel.cn/cn/coding-plan/latest-model)。模型概览逐项列出 1M/200K；Codex 示例列出 GLM-5.3 的 1,048,576 和 GLM-5-Turbo 的 204,800；套餐切换指南对 GLM-5.3/Flash 明确填写 1,000,000。对 1M 组统一采用较小的文档基数。

| 模型 | 官方标称 | 本次容量基数（token） | 80% 预设 |
| --- | --- | ---: | ---: |
| glm-5.3 | 1M | 1,000,000 | 800,000 |
| glm-5.3-flash | 1M | 1,000,000 | 800,000 |
| glm-5.2 | 1M | 1,000,000 | 800,000 |
| glm-5.1 | 200K | 204,800 | 163,840 |
| glm-5 | 200K | 204,800 | 163,840 |
| glm-5-turbo | 200K | 204,800 | 163,840 |
| glm-5v-turbo | 200K | 204,800 | 163,840 |
| glm-4.7 | 200K | 204,800 | 163,840 |
| glm-4.7-flashx | 200K | 204,800 | 163,840 |
| glm-4.7-flash | 200K | 204,800 | 163,840 |

Coding Plan 的 `glm-5.3`、`glm-5.3-flash` 同样为 1,000,000 → 800,000。

## DeepSeek

官方依据：[模型与兼容别名](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/)、[Codex 接入](https://api-docs.deepseek.com/quick_start/agent_integrations/codex/)。官方目录对当前 Flash 和 Pro 均明确填写 1,048,576。旧 Flash ID 仍接受，但实际由 V4.1-Flash 提供服务。

| 模型 | 官方容量（token） | 80% 预设 |
| --- | ---: | ---: |
| deepseek-flash | 1,048,576 | 838,860 |
| deepseek-v4-flash | 1,048,576 | 838,860 |
| deepseek-v4-flash-vision-exp | 1,048,576 | 838,860 |
| deepseek-v4-pro | 1,048,576 | 838,860 |

## Kimi

官方依据：[模型列表](https://platform.kimi.com/docs/models)、[K2.7 Code 与高速版](https://platform.kimi.com/docs/guide/kimi-k2-7-code-quickstart)、[开放平台 Codex 接入](https://platform.kimi.com/docs/guide/codex-kimi)、[套餐模型配置](https://www.kimi.com/code/docs/kimi-code/models.html)。开放平台 K3 的 Codex 示例明确指定 1,048,576；K2.7 Code、高速版和 K2.6 均标注 256K，套餐配置表明确 256K 为 262,144。

| 普通 API 模型 | 官方容量（token） | 80% 预设 |
| --- | ---: | ---: |
| kimi-k3 | 1,048,576 | 838,860 |
| kimi-k2.7-code | 262,144 | 209,715 |
| kimi-k2.7-code-highspeed | 262,144 | 209,715 |
| kimi-k2.6 | 262,144 | 209,715 |

| Coding 套餐模型 | 官方容量 / 权益 | 本次容量基数 | 80% 预设 |
| --- | --- | ---: | ---: |
| kimi-for-coding | 当前映射 K2.8 Preview，所有会员最高 1,048,576 | 1,048,576 | 838,860 |
| kimi-for-coding-highspeed | K2.7 Code 高速版，262,144 | 262,144 | 209,715 |
| k3 | Moderato 262,144；Allegretto 及以上 1,048,576 | 262,144 | 209,715 |
| k3-256k | 固定 262,144 | 262,144 | 209,715 |

本地无法根据 API Key 判断会员档位，因此 `k3` 按最低可调用档位的容量预留 20%，界面注明 Allegretto 及以上可手动改为 838,860。套餐的 `k3` 与普通 API 的 `kimi-k3` 分别处理。

同次核对发现套餐文档已更新：`kimi-for-coding` 映射 K2.8 Preview，支持 low/high/max、默认 max；套餐 `k3`/`k3-256k` 官方默认 high。同步纠正这三项相关预设，普通 API 的 `kimi-k3` 仍为默认 max。

## 千问

普通 API 逐一核对对应模型页的「上下文限制」。除 `qwen3.6-max-preview` 外，均明确为 1,000,000；不把最大输入长度 983,616 当作上下文总容量。

| 模型 / 官方来源 | 官方容量（token） | 80% 预设 |
| --- | ---: | ---: |
| [qwen3.8-max](https://help.aliyun.com/zh/model-studio/qwen3-8-max) | 1,000,000 | 800,000 |
| [qwen3.8-flash](https://help.aliyun.com/zh/model-studio/qwen3-8-flash) | 1,000,000 | 800,000 |
| [qwen3.8-2.4t-a95b](https://help.aliyun.com/zh/model-studio/qwen3-8-2-4t-a95b) | 1,000,000 | 800,000 |
| [qwen3.8-27b](https://help.aliyun.com/zh/model-studio/qwen3-8-27b) | 1,000,000 | 800,000 |
| [qwen3.7-max](https://help.aliyun.com/zh/model-studio/qwen3-7-max) | 1,000,000 | 800,000 |
| [qwen3.7-plus](https://help.aliyun.com/zh/model-studio/qwen3-7-plus) | 1,000,000 | 800,000 |
| [qwen3.7-flash](https://help.aliyun.com/zh/model-studio/qwen3-7-flash) | 1,000,000 | 800,000 |
| [qwen3.6-max-preview](https://help.aliyun.com/zh/model-studio/qwen3-6-max) | 262,144 | 209,715 |
| [qwen3.6-plus](https://help.aliyun.com/zh/model-studio/qwen3-6-plus) | 1,000,000 | 800,000 |
| [qwen3.6-flash](https://help.aliyun.com/zh/model-studio/qwen3-6-flash) | 1,000,000 | 800,000 |

Token Plan 的 `qwen3.8-max`、`qwen3.8-flash` 采用相同总容量 1,000,000 → 800,000。[官方套餐 Codex 示例](https://help.aliyun.com/zh/model-studio/codex) 已限制在思考模式输入长度 983,616；本次 800,000 低于该值。套餐地址、协议、候选列表均未变更。

## MiniMax

官方依据：[模型上下文规格表](https://platform.minimax.cn/docs/api-reference/text-anthropic-api)、[Chat Completions](https://platform.minimax.cn/docs/api-reference/text-chat-openai)、[Token Plan Codex 示例](https://platform.minimax.cn/docs/token-plan/codex)。规格表逐一给出以下 8 个 ID 的数值，套餐 Codex 示例也明确指定 M3 为 1,000,000。

| 模型 | 官方容量（token） | 80% 预设 |
| --- | ---: | ---: |
| MiniMax-M3 | 1,000,000 | 800,000 |
| MiniMax-M2.7 | 204,800 | 163,840 |
| MiniMax-M2.7-highspeed | 204,800 | 163,840 |
| MiniMax-M2.5 | 204,800 | 163,840 |
| MiniMax-M2.5-highspeed | 204,800 | 163,840 |
| MiniMax-M2.1 | 204,800 | 163,840 |
| MiniMax-M2.1-highspeed | 204,800 | 163,840 |
| MiniMax-M2 | 204,800 | 163,840 |

Token Plan 的 `MiniMax-M3` 同样为 1,000,000 → 800,000。

## 离线验证范围

检查所有普通与套餐候选的独立上下文值、外层默认值及 80% 向下取整；检查编辑和保存后不重复缩减、已有上下文不被覆盖；检查生成 Codex 目录时逐模型使用正确数值。构建及上游文件完整性检查不代表已完成厂商长上下文、工具或真实 Codex 自动压缩测试。

验证结果：桌面端 63 项测试通过，Rust 模型目录相关 4 项测试通过；桌面端与中英文文档站构建通过；CC Switch 固定上游文件及本地预设校验通过。文档站使用本地默认环境构建，未配置生产 `site`，因此该次构建不生成生产 sitemap；不涉及部署。
