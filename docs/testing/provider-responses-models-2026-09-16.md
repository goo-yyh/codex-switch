# 2026-09-16：全部预设模型的 Responses 连通实测

实测区间：北京时间 2026-09-16 17:38:41 至 2026-09-16 17:44:19。

**81 个模型，73 个返回完整文本，8 个未通过完整文本检查。完整文本中 68 个严格回答 OK，5 个附带其他文本。**

使用仓库 `.env` 中五家厂商的 Key；每家只使用产品当前默认地址。没有使用 Codex App、本地转换网关、Chat 回退、Token Plan / Coding 专用地址，也没有修改预设或用户配置。结论仅绑定本次地址、模型和账户。

## 方法与边界

- 发送 `POST <默认地址>/responses`，`input="Reply with exactly OK."`，`stream=false`，首轮 `max_output_tokens=256`；两个 GLM thinking 模型另以 1,024 额度复查。请求不携带用户对话内容。
- 每条请求超时 60 秒，不跟随 HTTP 重定向；首轮每家串行、不同厂商并行。
- 首轮 81 次；对错误与非精确文本补充 18 次复查。首轮和复查分别保留，下面状态采用最后一次结果。
- 连通标准：HTTP 200、`status=completed`、非空 message 文本；严格文本标准还要求去除首尾空白后恰好等于 `OK`。HTTP 200 本身不算通过。
- **只验证普通非流式 Responses 文本请求，不证明 SSE、工具调用、图片、长上下文、远程压缩或 Codex App 端到端兼容。**
- 报告只保存脱敏错误、用量、状态和合成请求的非精确文本片段，不保存密钥或原始完整响应。

## 汇总

| 厂商 | 模型数 | 返回完整文本 | 严格 OK | 未完成或错误 |
| --- | ---: | ---: | ---: | ---: |
| 千问 | 43 | 35 | 33 | 8 |
| MiniMax | 8 | 8 | 8 | 0 |
| 智谱 GLM | 22 | 22 | 19 | 0 |
| Kimi | 4 | 4 | 4 | 0 |
| DeepSeek | 4 | 4 | 4 | 0 |

## 未通过的模型

| 模型 | HTTP | 复查原因 |
| --- | ---: | --- |
| `qwen3.6-27b` | 400 | Unsupported model: 'qwen3.6-27b' |
| `qwen3-235b-a22b-instruct-2507` | 400 | <400> InternalError.Algo.InvalidParameter: Agent capabilities are not enabled for the current model. |
| `qwen3-14b` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'choices': [{'message': ...inish_reason': 'null'}]}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |
| `qwen-max` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'finish_reason': 'null', 'text': 'Hello'}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |
| `qwen-coder-plus` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'finish_reason': 'null', 'text': 'Hello'}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |
| `qwen-coder-turbo` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'finish_reason': 'null', 'text': 'Hello'}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |
| `qwq-plus` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'choices': [{'message': ...inish_reason': 'null'}]}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |
| `qwen-long` | 500 | 1 validation error for Event object   Field required [type=missing, input_value={'choices': [{'message': ...inish_reason': 'null'}]}, input_type=dict]     For further information visit https://errors.pydantic.dev/2.11/v/missing |

这些错误不证明 Chat Completions 可用；本轮没有发 Chat 请求。500 是上游返回的响应结构校验异常，不据此断言模型永久不支持 Responses。

## 复查与别名

- `glm-4.6v-flash` 首轮 HTTP 429（访问量过大），稍后复查成功；不能把首轮过载算作协议不支持。
- `deepseek-v4-flash`、`deepseek-v4-flash-vision-exp` 的响应 `model` 均为 `deepseek-flash`，其余成功模型返回的 model 与请求一致。

| 非精确文本模型 | 复查文本片段 |
| --- | --- |
| `qwen3-max` | OK. |
| `qwen-turbo` | OK. |
| `glm-4.1v-thinking-flashx` | 文本包含思考过程；未严格只返回 OK |
| `glm-4.1v-thinking-flash` | 文本包含思考过程；未严格只返回 OK |
| `glm-4-flashx-250414` | OK. |

部分请求报告的 output_tokens 超过请求中指定的额度（共 5 次）；此字段不能当作所有上游都严格遵守的费用上限。响应用量总计：输入 3231 tokens，输出 8012 tokens；失败响应缺少用量，不包含在此合计。

## 完整模型结果

| 厂商 | 模型 | 最新结果 | HTTP | 耗时 ms |
| --- | --- | --- | ---: | ---: |
| 千问 | `qwen3.8-max` | 完整响应；严格 OK | 200 | 5400 |
| 千问 | `qwen3.8-flash` | 完整响应；严格 OK | 200 | 2473 |
| 千问 | `qwen3.8-2.4t-a95b` | 完整响应；严格 OK | 200 | 3172 |
| 千问 | `qwen3.8-27b` | 完整响应；严格 OK | 200 | 1511 |
| 千问 | `qwen3.7-max` | 完整响应；严格 OK | 200 | 7592 |
| 千问 | `qwen3.7-plus` | 完整响应；严格 OK | 200 | 3685 |
| 千问 | `qwen3.7-flash` | 完整响应；严格 OK | 200 | 4062 |
| 千问 | `qwen3.6-max-preview` | 完整响应；严格 OK | 200 | 3959 |
| 千问 | `qwen3.6-plus` | 完整响应；严格 OK | 200 | 5300 |
| 千问 | `qwen3.6-flash` | 完整响应；严格 OK | 200 | 2945 |
| 千问 | `qwen3.6-35b-a3b` | 完整响应；严格 OK | 200 | 3094 |
| 千问 | `qwen3.6-27b` | 请求失败 | 400 | 1185 |
| 千问 | `qwen3.5-plus` | 完整响应；严格 OK | 200 | 6252 |
| 千问 | `qwen3.5-flash` | 完整响应；严格 OK | 200 | 2319 |
| 千问 | `qwen3.5-397b-a17b` | 完整响应；严格 OK | 200 | 4632 |
| 千问 | `qwen3.5-122b-a10b` | 完整响应；严格 OK | 200 | 2929 |
| 千问 | `qwen3.5-35b-a3b` | 完整响应；严格 OK | 200 | 2761 |
| 千问 | `qwen3.5-27b` | 完整响应；严格 OK | 200 | 20898 |
| 千问 | `qwen3-max` | 完整响应；非精确文本 | 200 | 2244 |
| 千问 | `qwen3-coder-next` | 完整响应；严格 OK | 200 | 1240 |
| 千问 | `qwen3-coder-plus` | 完整响应；严格 OK | 200 | 1270 |
| 千问 | `qwen3-coder-flash` | 完整响应；严格 OK | 200 | 1084 |
| 千问 | `qwen3-coder-480b-a35b-instruct` | 完整响应；严格 OK | 200 | 1126 |
| 千问 | `qwen3-coder-30b-a3b-instruct` | 完整响应；严格 OK | 200 | 1249 |
| 千问 | `qwen3-next-80b-a3b-instruct` | 完整响应；严格 OK | 200 | 1145 |
| 千问 | `qwen3-next-80b-a3b-thinking` | 完整响应；严格 OK | 200 | 3456 |
| 千问 | `qwen3-235b-a22b-instruct-2507` | 请求失败 | 400 | 1028 |
| 千问 | `qwen3-235b-a22b-thinking-2507` | 完整响应；严格 OK | 200 | 1683 |
| 千问 | `qwen3-235b-a22b` | 完整响应；严格 OK | 200 | 4770 |
| 千问 | `qwen3-30b-a3b-instruct-2507` | 完整响应；严格 OK | 200 | 1860 |
| 千问 | `qwen3-30b-a3b-thinking-2507` | 完整响应；严格 OK | 200 | 3190 |
| 千问 | `qwen3-30b-a3b` | 完整响应；严格 OK | 200 | 2106 |
| 千问 | `qwen3-32b` | 完整响应；严格 OK | 200 | 4066 |
| 千问 | `qwen3-14b` | 请求失败 | 500 | 1208 |
| 千问 | `qwen3-8b` | 完整响应；严格 OK | 200 | 8130 |
| 千问 | `qwen-max` | 请求失败 | 500 | 988 |
| 千问 | `qwen-plus` | 完整响应；严格 OK | 200 | 1413 |
| 千问 | `qwen-flash` | 完整响应；严格 OK | 200 | 1106 |
| 千问 | `qwen-turbo` | 完整响应；非精确文本 | 200 | 2038 |
| 千问 | `qwen-coder-plus` | 请求失败 | 500 | 2384 |
| 千问 | `qwen-coder-turbo` | 请求失败 | 500 | 1279 |
| 千问 | `qwq-plus` | 请求失败 | 500 | 1963 |
| 千问 | `qwen-long` | 请求失败 | 500 | 1385 |
| MiniMax | `MiniMax-M3` | 完整响应；严格 OK | 200 | 2727 |
| MiniMax | `MiniMax-M2.7` | 完整响应；严格 OK | 200 | 2275 |
| MiniMax | `MiniMax-M2.7-highspeed` | 完整响应；严格 OK | 200 | 2583 |
| MiniMax | `MiniMax-M2.5` | 完整响应；严格 OK | 200 | 2443 |
| MiniMax | `MiniMax-M2.5-highspeed` | 完整响应；严格 OK | 200 | 2529 |
| MiniMax | `MiniMax-M2.1` | 完整响应；严格 OK | 200 | 1809 |
| MiniMax | `MiniMax-M2.1-highspeed` | 完整响应；严格 OK | 200 | 2463 |
| MiniMax | `MiniMax-M2` | 完整响应；严格 OK | 200 | 1145 |
| 智谱 GLM | `glm-5.3` | 完整响应；严格 OK | 200 | 7638 |
| 智谱 GLM | `glm-5.3-flash` | 完整响应；严格 OK | 200 | 1713 |
| 智谱 GLM | `glm-5.2` | 完整响应；严格 OK | 200 | 2522 |
| 智谱 GLM | `glm-5.1` | 完整响应；严格 OK | 200 | 7170 |
| 智谱 GLM | `glm-5` | 完整响应；严格 OK | 200 | 3867 |
| 智谱 GLM | `glm-5-turbo` | 完整响应；严格 OK | 200 | 7498 |
| 智谱 GLM | `glm-5v-turbo` | 完整响应；严格 OK | 200 | 1515 |
| 智谱 GLM | `glm-4.7` | 完整响应；严格 OK | 200 | 5957 |
| 智谱 GLM | `glm-4.7-flashx` | 完整响应；严格 OK | 200 | 1989 |
| 智谱 GLM | `glm-4.7-flash` | 完整响应；严格 OK | 200 | 3087 |
| 智谱 GLM | `glm-4.6` | 完整响应；严格 OK | 200 | 58654 |
| 智谱 GLM | `glm-4.6v` | 完整响应；严格 OK | 200 | 14381 |
| 智谱 GLM | `glm-4.6v-flash` | 完整响应；严格 OK | 200 | 1356 |
| 智谱 GLM | `glm-4.5-air` | 完整响应；严格 OK | 200 | 10497 |
| 智谱 GLM | `glm-4.5-airx` | 完整响应；严格 OK | 200 | 2141 |
| 智谱 GLM | `glm-4.5-flash` | 完整响应；严格 OK | 200 | 19763 |
| 智谱 GLM | `glm-4.1v-thinking-flashx` | 完整响应；非精确文本 | 200 | 2341 |
| 智谱 GLM | `glm-4.1v-thinking-flash` | 完整响应；非精确文本 | 200 | 2154 |
| 智谱 GLM | `glm-4-long` | 完整响应；严格 OK | 200 | 586 |
| 智谱 GLM | `glm-4-flashx-250414` | 完整响应；非精确文本 | 200 | 715 |
| 智谱 GLM | `glm-4-flash-250414` | 完整响应；严格 OK | 200 | 382 |
| 智谱 GLM | `glm-4v-flash` | 完整响应；严格 OK | 200 | 585 |
| Kimi | `kimi-k3` | 完整响应；严格 OK | 200 | 6461 |
| Kimi | `kimi-k2.7-code` | 完整响应；严格 OK | 200 | 1618 |
| Kimi | `kimi-k2.7-code-highspeed` | 完整响应；严格 OK | 200 | 4211 |
| Kimi | `kimi-k2.6` | 完整响应；严格 OK | 200 | 1518 |
| DeepSeek | `deepseek-flash` | 完整响应；严格 OK | 200 | 1193 |
| DeepSeek | `deepseek-v4-flash` | 完整响应；严格 OK | 200 | 938 |
| DeepSeek | `deepseek-v4-flash-vision-exp` | 完整响应；严格 OK | 200 | 731 |
| DeepSeek | `deepseek-v4-pro` | 完整响应；严格 OK | 200 | 1510 |

## 原始脱敏证据与复现

- [provider-responses-models-2026-09-16.jsonl](provider-responses-models-2026-09-16.jsonl)
- [provider-responses-models-rechecks-2026-09-16.jsonl](provider-responses-models-rechecks-2026-09-16.jsonl)
- [provider-responses-zhipu-rechecks-2026-09-16.jsonl](provider-responses-zhipu-rechecks-2026-09-16.jsonl)

```sh
cargo run -p codex-switch-core --bin provider-responses-check -- --all
cargo run -p codex-switch-core --bin provider-responses-check -- --model qwen3.6-27b
cargo run -p codex-switch-core --bin provider-responses-check -- --model glm-4.1v-thinking-flash --max-output-tokens 1024
```

命令仅在显式执行时读取 `.env` 并请求上游，不由页面加载、保存配置或 CI 自动触发。退出码 0 表示严格文本校验全部通过；非精确文本、错误或未测试项退出码为 1。
