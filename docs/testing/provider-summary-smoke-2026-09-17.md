# 普通摘要最小验证（2026-09-17）

使用仓库 `.env` 已配置的五家测试密钥和当前预设默认模型，每个模型发送一次普通 Responses 流式请求。输入为 97 条、129,841 bytes 的合成历史，追加仓库固定的 Codex 摘要提示词。输出上限 2,048 tokens，超时 120 秒。

未启动 Codex 客户端，未修改真实配置或会话，未调用 `/responses/compact`，未发送 `compaction_trigger`。本地应用数据库无已保存配置，本轮使用仓库预设，不代表所有候选模型均已通过。

| 模型 | HTTP / 状态 | 摘要字节数 | 耗时 | 结果 |
| --- | --- | ---: | ---: | --- |
| qwen3.8-max | 200 / completed | 1,630 | 22.70 秒 | 通过 |
| MiniMax-M3 | 200 / completed | 2,132 | 4.88 秒 | 通过 |
| glm-5.3 | 200 / completed | 2,208 | 11.72 秒 | 通过 |
| kimi-k3 | 200 / completed | 1,975 | 28.32 秒 | 通过 |
| deepseek-v4-flash | 200 / completed | 1,948 | 4.97 秒 | 通过 |

五项均满足：HTTP 成功、`status=completed`、摘要非空、流式增量文本与最终文本一致、摘要短于输入。只验证普通摘要生成，不验证摘要事实完整性、续接召回或 Codex 自动触发与指令重新注入。

脱敏回执：`reports/local/provider-compact-1789621180.jsonl`（本机保留；不包含密钥、原始响应或摘要正文）。

复跑命令（会调用真实服务）：

```sh
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke --all
```

测试工具的两项离线测试和针对该二进制的 Clippy 检查通过。
