---
title: Coding Plans and custom APIs
description: Connect custom HTTPS services to Codex App using Responses or Chat Completions, with base URLs, full endpoints and per-model overrides.
---

For GLM, MiniMax, Qwen and Kimi, use the **服务套餐 (Service plan)** selector to choose a dedicated plan. Providers with only one service do not show this selector. For services without a preset, choose **coding plan** when adding a configuration and enter its name, URL, API format, key and model IDs.

## Standard APIs and plans

Standard APIs, Coding Plans, overseas endpoints and enterprise gateways may use different keys, URLs and models. Use the matching values from the provider, even when keys look alike.

The project currently includes these additional plan presets:

| Plan | Base URL | API format |
| --- | --- | --- |
| GLM Coding Plan | `https://open.bigmodel.cn/api/coding/paas/v4` | Chat Completions |
| MiniMax Token Plan | `https://api.minimax.cn/v1` | Responses |
| Kimi Coding | `https://api.kimi.com/coding/v1` | Responses |
| Qwen Token Plan | `https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1` | Responses |

Choosing a plan loads its initial URL, models and capabilities and clears the key being entered. Account permissions depend on the provider. Updating presets does not overwrite saved configurations.

The GLM plan preset offers `glm-5.3` and `glm-5.3-flash` through the official Chat endpoint, with local protocol conversion. GLM also documents `https://open.bigmodel.cn/api/v1` for Codex Responses requests; this matches the existing GLM preset. [Official model switching guide](https://docs.bigmodel.cn/cn/coding-plan/latest-model)

MiniMax currently calls its subscription **Token Plan**. Its Codex guide uses the Responses URL above with `MiniMax-M3`. Enter the subscription key from subscription management; it is separate from a pay-as-you-go API key. [Official integration guide](https://platform.minimax.cn/docs/token-plan/codex), [key requirements](https://platform.minimax.cn/docs/token-plan/quickstart)

These new presets follow official documentation and have not been tested with real subscription keys. The dropdown lists actual plans only; edit the URL and API format fields directly when needed.

## Which URL to enter

| Supplied address  | Configuration                                                                                 | Ordinary request target                       |
| ----------------- | --------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Responses base    | `https://api.example.com/prefix/v1`, full URL off, Responses selected                         | `https://api.example.com/prefix/v1/responses` |
| Chat base         | `https://api.example.com/v1`, full URL off, Chat Completions selected                         | `https://api.example.com/v1/chat/completions` |
| Complete endpoint | `https://api.example.com/gateway/generate?version=1`, full URL on, actual API format selected | The entered URL, unchanged                    |

Base-URL mode preserves path prefixes and does not duplicate `/v1`. Recognized `/responses` and `/chat/completions` suffixes are normalized when pasted. Full-URL mode preserves paths and query parameters without appending the ordinary request path.

Only external HTTPS upstreams are accepted, not HTTP or loopback addresses. URLs cannot include a username, password or `#` fragment; base-URL mode also rejects query parameters. Enter API keys in the separate key field.

In full-URL mode, a legacy remote compact path can only be inferred from a recognized `/responses` ending. Arbitrary endpoints may not support it. Working ordinary requests do not prove remote compaction support; keep [remote context compaction](/en/docs/settings/#remote-context-compaction) off unless needed and supported.

## Choose the API format

- **Responses:** use when the service explicitly offers Responses API. The local gateway replaces model identifiers and forwards requests.
- **Chat Completions:** use for Chat-only services. CC Switch conversion modules translate text, images and tool calls into Responses events. See [compatibility](/en/docs/compatibility/).

Failures do not trigger automatic protocol changes, domain changes or fallback configurations. Check keys and permissions for 401 / 403; check plans, model IDs and paths for 404.

## Per-model overrides and testing

Models inherit outer connection settings by default. **编辑 (Edit)** allows individual overrides. Both tests and live routing use each model’s effective settings. URL or full-URL mode changes require re-entering the key. See [configuration](/en/docs/configuration/).

Requests follow **Codex → local loopback router → your configured service**. Codex Switch does not operate a cloud relay; the selected provider’s policies govern its handling of request data.
