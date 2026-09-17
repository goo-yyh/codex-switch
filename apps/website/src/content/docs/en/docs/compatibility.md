---
title: API compatibility and validation scope
description: Understand Responses and Chat Completions support, tools, streaming, ordinary summaries and remote compaction limits in Codex Switch.
---

This is a developer preview. Protocol conversion, local tests and provider HTTP checks establish different things. A model returning text does not prove complete Codex App compatibility.

## API capabilities

| Capability                       | Native Responses upstream                         | Chat Completions upstream                                  |
| -------------------------------- | ------------------------------------------------- | ---------------------------------------------------------- |
| Text and streamed text           | Forwarded                                         | Converted locally                                          |
| Function tools and results       | Forwarded                                         | Format conversion and multi-turn history                   |
| Namespace / Custom tools         | Forwarded, depends on upstream                    | Flattened / JSON-wrapped, then restored                    |
| Image input                      | Forwarded according to model capabilities         | Converted according to declared support                    |
| Hosted tools such as web search  | Depends on upstream; not enabled                  | Rejected with an explanation                               |
| Ordinary text summaries          | Forwarded as ordinary requests                    | Ordinary Chat conversion                                   |
| Remote compaction                | Forwarded when enabled; requires upstream support | No Remote V2 trigger items or encrypted compaction history |
| WebSocket                        | Not enabled                                       | Not enabled                                                |
| Automatic fallback after failure | Not supported                                     | Not supported                                              |

Model catalogs reference and reuse CC Switch templates and its capability registry, with presets maintained here. [Per-model declarations](/en/docs/configuration/) do not change the actual upstream capabilities.

## Remote compaction and ordinary summaries

[Remote context compaction](/en/docs/settings/#remote-context-compaction) is off by default and should generally stay off. Ordinary requests can still generate text summaries.

When enabled, Codex selects the remote protocol. Native Responses routes forward Remote V2 `compaction_trigger` fields, compaction items and legacy `/responses/compact` requests. Check upstream support separately; automatic fallback is not guaranteed.

Chat routes reject Remote V2 triggers and opaque encrypted compaction history. Although legacy Chat compact follows the ordinary upstream conversion path, that does not implement the remote compaction contract. It does not fabricate encrypted compaction items.

## Ordinary summary check

On 2026-09-17, each of the five current preset defaults received **one ordinary streaming Responses request**, with about 128 KB of synthetic history and a 2,048-token output limit.

| Model               | Result                                |
| ------------------- | ------------------------------------- |
| `qwen3.8-max`       | HTTP 200, completed, nonempty summary |
| `MiniMax-M3`        | HTTP 200, completed, nonempty summary |
| `glm-5.3`           | HTTP 200, completed, nonempty summary |
| `kimi-k3`           | HTTP 200, completed, nonempty summary |
| `deepseek-v4-flash` | HTTP 200, completed, nonempty summary |

For all five, streamed text matched the final text and the summary was shorter than the input. See the [summary smoke report](https://github.com/goo-yyh/codex-switch/blob/main/docs/testing/provider-summary-smoke-2026-09-17.md).

This check **did not launch Codex App or call remote compact**. It did not test factual completeness, continuation recall, automatic triggering or instruction reinjection. It does not cover every candidate model, Coding Plan, overseas endpoint or enterprise gateway.

Other results are in the [historical test records](https://github.com/goo-yyh/codex-switch/tree/main/docs/testing). Evidence applies only to its recorded date, model, endpoint and test. New presets do not inherit old models’ results.

## Local session cache

Chat conversion history and tool metadata are held only in process memory: up to 64 entries, 1 MiB per entry and one hour, isolated by connection. The router supports `previous_response_id`, unambiguous `call_id` completion within a connection, and complete tool history returned by the client.

Conflicting identifiers are not guessed. After a restart, timeout or eviction, references that cannot be reconstructed produce an explicit error; start a new task. Codex history is not read from disk to fill requests.

## Not yet validated

Offline tests use temporary directories, mock credentials and synthetic requests. Provider tests are separate, explicit HTTP checks. Real Codex App long-task end-to-end validation, Windows hardware testing, release signing and notarization remain pending.

Screenshots show the current interface only. Connection, backup and process states in the browser preview are simulated.
