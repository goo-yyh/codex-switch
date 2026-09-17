---
title: Qwen, MiniMax, GLM, Kimi and DeepSeek
description: Connect five built-in model providers to Codex App, select multiple models and distinguish standard API credentials from Coding Plan credentials.
---

Presets reference a pinned CC Switch revision; this project maintains the candidate models. They are initial values for new configurations, not proof of account access, and do not overwrite saved URLs, protocols or models.

| Provider | Default model       | API URL                                             | Format    |
| -------- | ------------------- | --------------------------------------------------- | --------- |
| Qwen     | `qwen3.8-max`       | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Responses |
| MiniMax  | `MiniMax-M3`        | `https://api.minimaxi.com/v1`                       | Responses |
| GLM      | `glm-5.3`           | `https://open.bigmodel.cn/api/v1`                   | Responses |
| Kimi     | `kimi-k3`           | `https://api.moonshot.cn/v1`                        | Responses |
| DeepSeek | `deepseek-v4-flash` | `https://api.deepseek.com`                          | Responses |

The service plan selector also offers GLM Coding Plan, MiniMax Token Plan, Kimi Coding and Qwen Token Plan. Selecting a plan loads its URL, models and capabilities and clears the key being entered. All provider URLs can be edited.

Context size, reasoning levels and image support come from model presets. Select a model and click **编辑 (Edit)** to adjust them. Confirm the dialog and then save the configuration; cancelling discards the dialog changes. Unknown models use CC Switch templates and its capability registry, which should be adjusted to match the service.

## Multiple models in one configuration

Enter one key and select 1–20 models. **保存配置 (Save configuration)** stores a named configuration without testing. Disable Codex Switch before changing its model list. When URLs and full-URL mode are unchanged, leaving the key blank preserves it.

Select configurations from several providers together on the home screen. Models appear as `configuration-name-model`. **已配置 (Already configured)** indicates reuse in another configuration with the same provider, URL and format; changing the current selection does not delete models elsewhere.

You can also enter other model IDs. The list is not an account entitlement list; model and plan access are controlled by the provider.

## Standard APIs and Coding Plans

These products may use different keys, URLs and models. Similar key formats do not make them interchangeable. For 401, 403 or 404 errors, verify the plan in the provider console. Do not send the same key to unknown third-party endpoints.

## Official documentation

- [Qwen](https://help.aliyun.com/zh/model-studio/)
- [MiniMax](https://platform.minimaxi.com/docs/api-reference/text-openai-api)
- [GLM](https://docs.bigmodel.cn/)
- [Kimi](https://platform.moonshot.cn/docs/)
- [DeepSeek](https://api-docs.deepseek.com/)

## What a test establishes

**测试配置 (Test configuration)** checks each selected model’s basic text response and identifies failures. It does not save or enable anything, and saving does not require a passing test. Tools, multi-turn context and streaming have separate protocol checks. Provider-specific evidence must come from the relevant test records, not a general connectivity label.

See [configuration and capabilities](/en/docs/configuration/) for overrides and [settings](/en/docs/settings/) for remote compaction.
