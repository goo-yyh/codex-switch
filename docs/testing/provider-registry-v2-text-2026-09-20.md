# 数据版本 2：变更模型文本连通实测

时间：2026-09-20 15:43（北京时间）。用户明确授权使用仓库 `.env` 密钥测试。

使用各厂商当前普通服务默认地址，分别调用新增的两个模型和调整后的 DeepSeek 默认模型。共 3 次请求，无重试、无 Chat 回退、无套餐端点请求。不读取或发送用户对话。

请求：`POST /responses`，`input="Reply with exactly OK."`，`stream=false`，`max_output_tokens=1024`，超时 60 秒，不跟随重定向；未显式覆盖思考档位，使用服务端默认值。

| 厂商 | 模型 | HTTP | 完成状态 | 文本 | 耗时 |
| --- | --- | --- | --- | --- | --- |
| 千问 | `qwen3.8-omni-flash` | 200 | completed | OK | 4.353 秒 |
| 智谱 | `glm-5.3-flashx` | 200 | completed | OK | 2.116 秒 |
| DeepSeek | `deepseek-flash` | 200 | completed | OK | 2.042 秒 |

三条响应均满足成功 HTTP 状态、`status=completed`、非空 message 文本，并严格返回 `OK`。响应 model 与请求一致。

官方响应报告用量合计：输入 141 tokens，输出 109 tokens，共 250 tokens。仅保留[脱敏结果](./provider-registry-v2-text-2026-09-20.jsonl)，不保存密钥或完整原始响应。

验证范围仅限本次账户、地址及普通非流式文本请求，不代表 SSE、工具调用、图片、长上下文、远程压缩、套餐或 Codex App 端到端已通过；其余未变更候选未在本次重新测试。
