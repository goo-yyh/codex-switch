# 模型思考档位核对（2026-09-17）

范围：`models.json` 的全部 36 个内置候选及现有套餐候选。只查阅官方文档并运行离线测试，未读取真实密钥、调用厂商接口或修改用户配置。此次不统一设为 high，以对应接口和套餐的官方说明为准。

## 结果

| 厂商 / 模型 | 客户端可选值 | 预设默认值 | 说明 / 官方依据 |
| --- | --- | --- | --- |
| 千问 3.8：max、flash、2.4t-a95b、27b | none, low, medium, xhigh | xhigh | [Responses 参数](https://help.aliyun.com/zh/model-studio/qwen-api-via-openai-responses) 按系列说明；high、max 是 xhigh 的兼容映射，不重复列入 |
| 千问 3.7-max、3.7-plus、3.6-plus、3.6-flash | none, low, medium, high, xhigh | medium | [官方 Codex 模型目录示例](https://help.aliyun.com/zh/model-studio/codex)；Responses 支持 none 关闭 |
| 千问 3.7-flash、3.6-max-preview | none, low, medium, high, xhigh | 未填，自动选择 | 官方 Responses 公共接口允许这些值，但本次未查到这两个 ID 单独公布的默认档位；不将其他型号的 medium 当作已核实的官方默认 |
| GLM 5.3、5.3-flash | low, high, max | max | [参数说明](https://docs.bigmodel.cn/cn/guide/start/concept-param)、[最新套餐指南](https://docs.bigmodel.cn/cn/coding-plan/latest-model)；强制思考 |
| GLM 5.2 | none, high, max | max | [深度思考](https://docs.bigmodel.cn/cn/guide/capabilities/thinking)；省略兼容别名：minimal→关闭，low/medium→high，xhigh→max |
| GLM 5.1、5、5-turbo、5v-turbo、4.7、4.7-flashx、4.7-flash | none, high（开关表示） | high（开启） | 官方仅公布 thinking 开关，默认开启；没有原生多档 effort。移除 5-turbo 原先只有 max 的错误声明 |
| MiniMax M3 普通 Responses | none, high（开关表示） | none | [Responses API](https://platform.minimax.cn/docs/api-reference/responses-create) 默认关闭；high 开启 Adaptive Thinking，不代表深度分档 |
| MiniMax M3 Token Plan / Codex 预设 | none, high（开关表示） | high | [官方 Codex 接入示例](https://platform.minimax.cn/docs/token-plan/codex) 显式使用 high；与裸 API 缺省行为分开记录 |
| MiniMax M2.7、M2.7-highspeed、M2.5、M2.5-highspeed、M2.1、M2.1-highspeed、M2 | high（固定开启） | high | 官方 Responses 文档说明 M2.x 无法关闭推理；不提供 none 或虚构 low/max 深度档位 |
| Kimi K3（普通 API） | low, high, max | max | [Chat 参数](https://platform.kimi.com/docs/api/chat)、[Responses 参数](https://platform.kimi.com/docs/api/responses)；始终思考 |
| Kimi K2.7 Code / Highspeed、套餐 kimi-for-coding-highspeed | high（固定开启） | high | [K2.7 Code 指南](https://platform.kimi.com/docs/guide/kimi-k2-7-code-quickstart)；不支持非思考模式 |
| Kimi K2.6 | none, high（开关表示） | high（开启） | 同上，thinking 默认 enabled，可关闭 |
| DeepSeek Flash、V4-Flash、V4-Flash-Vision-Exp、V4-Pro | none, low, high, max | high | [思考模式](https://api-docs.deepseek.com/guides/thinking_mode/)、[官方模型与别名](https://api-docs.deepseek.com/quick_start/pricing/) |

## 原生档位与客户端兼容表示

`none/high` 对开关型模型只表达关闭/开启，不声称厂商支持这两个深度等级。界面会显示说明。MiniMax M2.x 和 Kimi K2.7 Code 只保留开启状态。实际 Responses 请求参数是否被历史型号的上游接口接受，需要分别用对应账户验证；文档核对不能代替实际 API 测试。

官方未单列默认的两个千问型号保留“自动选择”，生成目录仍走既有客户端兼容模板；这不是对上游默认值的声明。未修改固定版本的 CC Switch 转换模块。

## 已有配置

编辑器和模型目录生成器仅为缺失的思考字段补充预设，保留已填写的档位、默认值、地址、上下文长度等。不会静默改写数据库，也无法从历史数据判断某个已保存的 max/high 是旧预设还是用户手填。已有明确值要改用新默认时，需在模型编辑中修改并保存。

当用户自定义可选列表排除了官方默认值且没有选择默认档位时，继续采用列表的最后一项作为兼容默认；不向该列表注入不支持的值。未知自定义模型保持原行为。

## 同日上下文核对时更新套餐映射

[最新套餐规格](https://www.kimi.com/code/docs/kimi-code/models.html)明确 `k3` / `k3-256k` 的 low/high/max 默认 high；`kimi-for-coding` 已映射到 K2.8 Preview，支持 low/high/max、默认 max。已纠正先前按开放平台 K3 与 K2.7 推断的套餐配置。普通 API 的 K3 默认 max 不变，已有明确保存值保留。
