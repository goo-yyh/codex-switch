---
title: Coding Plans and custom APIs
description: Connect custom HTTPS services to Codex App using Responses or Chat Completions, with base URLs, full endpoints and per-model overrides.
---

For built-in providers, use the **服务套餐 (Service plan)** selector. For services without a preset, choose **coding plan** when adding a configuration and enter its name, URL, API format, key and model IDs.

## Standard APIs and plans

Standard APIs, Coding Plans, overseas endpoints and enterprise gateways may use different keys, URLs and models. Use the matching values from the provider, even when keys look alike.

The project currently includes these additional plan presets:

| Plan            | Base URL                                                             |
| --------------- | -------------------------------------------------------------------- |
| Kimi Coding     | `https://api.kimi.com/coding/v1`                                     |
| Qwen Token Plan | `https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1` |

Choosing a plan loads its initial URL, models and capabilities and clears the key being entered. Account permissions depend on the provider. Updating presets does not overwrite saved configurations.

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
