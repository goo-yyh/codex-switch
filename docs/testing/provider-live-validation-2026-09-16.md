# 2026-09-16：真实请求与长上下文压缩验证

> 范围澄清：本文的远程压缩测试仅覆盖独立 `/responses/compact` 接口，不覆盖新版 Remote V2 的 `/responses` + `compaction_trigger`，也不代表普通摘要压缩失败。后续默认路径的结果见 [普通摘要压缩验证](provider-summary-validation-2026-09-16.md)。

实测时间：北京时间 2026-09-16 15:37:38–15:42:48。使用仓库 `.env` 中五家供应商的 Key，直接访问当前预设的原生 Responses 接口。未经过 Codex App、CLI、app-server 或本地网关；未修改产品预设、真实 Codex 配置或登录态。

**结论：四家通过文本、流式和工具闭环校验；本轮五家均未通过完整的远程压缩续接验收。MiniMax 能返回压缩项，但后续请求不能找回测试事实，不能据此开启远程压缩。**

## 实测矩阵

| 供应商 / 当前默认模型        | 文本       | SSE  | 工具调用及回传                    | `/responses/compact`                                               | 压缩后续接                           |
| ---------------------------- | ---------- | ---- | --------------------------------- | ------------------------------------------------------------------ | ------------------------------------ |
| 千问 `qwen3.8-max`           | 通过       | 通过 | `auto` 调用 + `none` 回传通过     | HTTP 405                                                           | 因接口失败跳过                       |
| MiniMax `MiniMax-M3`         | 通过       | 通过 | `required` 调用 + `none` 回传通过 | HTTP 200；返回真实接口提供的 compaction 项和非空 encrypted_content | 失败：长上下文 0/6；两组短对照均 0/1 |
| 智谱 `glm-5.3`               | 120 秒超时 | 跳过 | 跳过                              | HTTP 501，`Compact is not implemented yet`                         | 因接口失败跳过                       |
| Kimi `kimi-k3`               | 通过       | 通过 | 调用和回传均使用 `auto` 时通过    | HTTP 404                                                           | 因接口失败跳过                       |
| DeepSeek `deepseek-v4-flash` | 通过       | 通过 | `auto` 调用 + `none` 回传通过     | HTTP 404                                                           | 因接口失败跳过                       |

“通过”包括响应完成、精确文本匹配，SSE 增量与最终文本一致，以及虚拟函数名称、参数、call_id、结果回传校验。不是仅检查 HTTP 200。

本轮地址限定如下，未尝试 Token Plan / Coding 套餐入口或其他模型：

| 供应商   | Base URL                                            |
| -------- | --------------------------------------------------- |
| 千问     | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| MiniMax  | `https://api.minimaxi.com/v1`                       |
| 智谱     | `https://open.bigmodel.cn/api/v1`                   |
| Kimi     | `https://api.moonshot.cn/v1`                        |
| DeepSeek | `https://api.deepseek.com`                          |

分别在 Base URL 后追加 `/responses` 和 `/responses/compact`。结论绑定本次端点、模型和账户，不能外推为供应商全部产品均不支持。

## MiniMax 长上下文压缩

历史为 129,841 字节、97 条消息；六个随机校验值分别位于 assistant 历史的前、中、后段。另含一个后续被覆盖的旧 release_code。用户续接问题和系统指令均不携带答案。

| 阶段                 | HTTP / 状态               | 输入 tokens | 输出 tokens |     耗时 | 校验                    |
| -------------------- | ------------------------- | ----------: | ----------: | -------: | ----------------------- |
| 原始历史直接回答     | 200 / completed           |      23,120 |          75 | 7.148 秒 | 6/6，旧值覆盖正确       |
| `/responses/compact` | 200 / response.compaction |      23,328 |         581 | 5.116 秒 | 返回 1 个非空加密压缩项 |
| 仅凭返回窗口续接     | 200 / completed           |         230 |          37 | 3.073 秒 | 0/6，JSON 格式合法      |

下一次输入严格使用 `compact.output` 原样加上新问题，没有裁剪输出、补回原始历史或使用 previous_response_id。供应商报告的续接输入 token 减少约 99.0%，但事实召回完全失败，**整体压缩验收失败**。token 数量下降本身不能证明上下文可用。

两个额外边界检查：

1. 短合成历史压缩返回 200，续接为 completed，但精确校验 0/1。
2. 使用服务端真实生成的 assistant 输出组成历史；初始生成与续接均开启 `reasoning.effort=high`。原始响应精确校验 1/1，压缩返回 200；续接正常完成但 `probe_code` 为 null，校验 0/1。续接输出 145 tokens，其中推理 131 tokens，没有达到 2,048 上限。

因此，这次失败不能归因于答案截断，也不能仅归因于手工构造的 assistant 历史或默认关闭推理。尚未定位到供应商内部的具体原因；不能断言服务端究竟在摘要、压缩项解析还是续接处理中丢失了状态。未解码或改写供应商返回的 opaque encrypted_content。

## 工具参数兼容差异

- 千问在默认思考模式下拒绝 `tool_choice=required`，返回 400。改为 auto 后，调用和结果回传通过。
- DeepSeek 在默认思考模式下同样拒绝 required。改为 auto 后，调用和结果回传通过。
- Kimi 分别拒绝 required 和 none，均返回 400。调用及回传都使用 auto 时闭环通过。
- MiniMax 在本次请求中接受 required 和 none，并完成闭环。这是实测结果，不代表所有接口或版本都承诺支持相同参数。

这些失败有明确参数原因，不能报告为模型不支持工具。诊断仅调整独立脚本请求，没有静默修改产品转发参数或切换 Chat 协议。

## 请求预算与证据

初始预算每家最多 9 次请求，普通生成输出上限 2,048 tokens；普通请求超时 120 秒，压缩超时 180 秒。压缩请求未发送契约之外的输出上限。普通请求默认不传 reasoning，使用供应商默认行为；仅 MiniMax 对照明确设置 high。所有请求串行、无自动重试。

鉴于初轮暴露的问题，明确追加了工具参数诊断和 MiniMax 的 3 次小规模对照。最终共 34 次请求：

| 供应商   | 请求数 | 返回 usage 的请求数 | 报告输入 tokens | 报告输出 tokens | 报告合计 tokens |
| -------- | -----: | ------------------: | --------------: | --------------: | --------------: |
| 千问     |      6 |                   4 |             840 |             110 |             950 |
| MiniMax  |     12 |                  12 |          49,699 |           1,371 |          51,070 |
| 智谱     |      2 |                   0 |            未知 |            未知 |            未知 |
| Kimi     |      8 |                   5 |           1,085 |             417 |           1,502 |
| DeepSeek |      6 |                   4 |             626 |             132 |             758 |

已返回 usage 的请求合计 54,280 tokens；错误或超时请求的用量未知，不按零消耗解释，也不据此推算最终账单。

- [脱敏结构化回执](provider-live-validation-2026-09-16.json)：保留所有四轮运行的状态、用量、耗时、输出类型、哈希和校验结果，包括失败。
- 脚本：`crates/core/src/bin/provider-compact-check.rs`；入口：`pnpm test:providers:compact --all`。
- 诊断入口：`pnpm test:providers:compact --tools-auto kimi`、`pnpm test:providers:compact --compact-control minimax`。这些命令会产生真实调用及费用，不属于常规 CI。
- 原始运行的脱敏 JSONL 位于已忽略的 `reports/local/provider-compact-*.jsonl`，权限 0600。未记录 Key、输出正文、推理正文或加密压缩原文。
- 脚本的两个离线测试通过，验证召回判定不会将不完整/错误结果当成功，且随机答案不会泄漏进续接问题或用户历史。该二进制的 Clippy 严格检查通过。

最后一轮 tools-auto 模式将回传参数统一为 auto；前一轮千问和 DeepSeek 的回传使用 none。结构化回执中的运行顺序与本报告保留这一差异，未覆盖失败记录。

## 对后续开发的影响与验证边界

当前证据支持保留原生 Responses 作为四家模型的可用请求方式，但**不能把“支持 Responses”推导为“支持远程压缩”**。远程压缩默认关闭仍符合本轮证据；MiniMax 也不应仅凭 compact 返回 200 就被标记为完整支持。

后续接入应保留独立的普通请求、工具参数和压缩续接能力判定。供应商错误或超时应显示真实原因；不能伪造加密压缩项或悄悄变更协议来表示压缩成功。

本轮尚未证明：智谱普通请求可用、百万 token 上下文上限、流式工具调用、长链多次压缩、自动故障切换、真实 Codex 客户端的压缩触发/回退行为。长上下文实测约 2.3 万 tokens，为单个合成样本；未对失败接口重复上传长历史。

契约依据：[OpenAI Compaction](https://developers.openai.com/api/docs/guides/compaction) 要求原样续接完整返回窗口；[MiniMax Responses 文档](https://platform.minimaxi.com/docs/api-reference/responses-create) 说明 M3 默认关闭推理，high 会开启推理。上述支持性结论以本次真实请求回执为准。
