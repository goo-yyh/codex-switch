---
title: CC Switch and open-source credits
description: Learn which CC Switch modules Codex Switch reuses, the pinned upstream revision, MIT attribution and the code maintained independently here.
---

Codex Switch references and directly reuses portions of [CC Switch](https://github.com/farion1231/cc-switch). Thanks to Jason Young and the CC Switch community.

## Reused code

| Area                                      | Usage                                            |
| ----------------------------------------- | ------------------------------------------------ |
| Chat / Responses conversion               | Request and response format adaptation           |
| SSE processing                            | Streamed text, reasoning and tool events         |
| Tool history and schema helpers           | Tool history completion and format handling      |
| Model catalog and capability templates    | Model metadata and capability defaults           |
| JSON / media helpers and provider presets | Selected helper logic and initial values         |
| Related tests                             | Regression coverage for conversion and streaming |

The upstream revision is pinned to [`06082e189d65e6d6dbadc35dacdac1ce6c79d89a`](https://github.com/farion1231/cc-switch/tree/06082e189d65e6d6dbadc35dacdac1ce6c79d89a). Code lives in `crates/cc-switch-codex`; SHA-256 manifests track source files and extracted fragments.

## Maintained by Codex Switch

The desktop interface, configuration selection, activation transactions, backups and recovery, credential integration, local routing, provider presets, native integration and this documentation are maintained in this repository.

These are extracted source modules, not an official CC Switch SDK. The upstream database and general router are not imported wholesale; its retained circuit breaker is not used by the local gateway. Failed requests do not automatically switch services or protocols. Local adaptations include a bounded session cache and request validation.

See [UPSTREAM.md](https://github.com/goo-yyh/codex-switch/blob/main/crates/cc-switch-codex/UPSTREAM.md) for exact reuse boundaries, differences and update rules. `pnpm check:upstream` verifies the pinned sources and attribution.

## License and independence

This project uses the [MIT license](https://github.com/goo-yyh/codex-switch/blob/main/LICENSE). Reused CC Switch code retains its original [MIT license and copyright notice](https://github.com/goo-yyh/codex-switch/blob/main/crates/cc-switch-codex/LICENSE): Copyright (c) 2025 Jason Young.

Codex Switch is an independent community project, not an official OpenAI product and not affiliated with OpenAI. Referencing CC Switch does not imply endorsement by its authors or inherit its test results. See this project’s [validation scope](/en/docs/compatibility/).
