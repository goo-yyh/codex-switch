---
title: Connect Codex App to your models
description: Manage DeepSeek, Kimi, Qwen, GLM, MiniMax and custom APIs in Codex App with multiple configurations, local key storage and configuration recovery.
---

Codex Switch is an independent desktop app for managing model connections in **Codex App**. Each configuration holds one API key and multiple models. Select several configurations together, back up the original settings before enabling, and restore them when you switch off.

[![Codex Switch with example Kimi and DeepSeek configurations enabled](/screenshots/enabled.png)](/screenshots/enabled.png)

_Actual app interface in Chinese, captured in the browser preview with example configurations. Connection and backup states are simulated._

## Start here

[Installation](/en/docs/install/) → [Screenshot walkthrough](/en/docs/quickstart/) → [Configuration and capabilities](/en/docs/configuration/).

## What you can do

- Connect Qwen, MiniMax, GLM, Kimi, DeepSeek, or [plans and custom HTTPS APIs](/en/docs/relay/).
- Save multiple configurations with 1–20 models each, including per-model URL, API format and capability overrides.
- Test connections on demand, then select models in Codex as `configuration-name-model`.
- Store keys in the system credential store and route requests locally, without a Codex Switch cloud service.
- Manage background connections from the tray and [restore the original configuration](/en/docs/switch/) when switching off.

## Current status

Version 0.1.0 is a developer preview targeting macOS and Windows. Public signed installers are not yet available. Local tests, builds and provider HTTP checks do not establish complete Codex App compatibility. Read the [compatibility notes](/en/docs/compatibility/) first.

Codex tools may share a configuration directory, so other clients using that directory may also read these settings. CLI and IDE extension compatibility is not guaranteed.

## Open-source origins

This project references and directly reuses protocol conversion, streaming, tool history and model catalog code from [CC Switch](https://github.com/farion1231/cc-switch), preserving its MIT copyright notices. See [open-source credits](/en/docs/open-source/) for the pinned revision and maintenance scope.
