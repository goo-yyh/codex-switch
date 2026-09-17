---
title: Changelog and preview release status
description: Follow Codex Switch development, model connection features, compaction settings, bilingual documentation and outstanding release validation.
---

## 0.1.0 · Developer preview

- A focused connection manager for Codex App.
- Presets for five model providers and custom APIs.
- A main switch, pre-activation backups, exact restoration and external-edit conflict protection.
- Responses forwarding and Chat text, streaming and tool conversion.
- System credential storage, model routing aliases, tray controls and startup preferences.
- Normal launch and restart actions, plus manual recovery after an unexpected exit.
- A standalone website, product docs, offline tests and explicit provider-test commands.

### 2026-09-17 · Configuration and documentation

- Removed the fallback queue and routing save action; failed requests no longer automatically switch services.
- Remote context compaction is off by default. Its toggle saves automatically, persists after reopening and applies at the next activation.
- Connections support full URLs, with per-model URL, format and capability overrides.
- Added a minimal ordinary-summary check: one request per provider default, without launching Codex.
- Updated the README and website with actual interface screenshots, configuration details and CC Switch attribution.
- Added Chinese and English website and documentation pages, equivalent-page language switching, localized SEO metadata, structured data and sitemap validation.

The entries below describe historical changes. Current behavior is defined by the usage guides.

### 2026-09-15 · Review fixes

- Fixed old-key reuse across provider or plan URLs; changing keys creates a new routing revision and retains credentials for existing tasks at that revision.
- Fixed streamed tool history completion, long namespace tool names and parallel tool argument conversion.
- Fixed leaving the page during validation and cancellation races; validation-and-save no longer switches the default connection.
- Added activation-failure compensation, distinct credential errors, connection-only recovery and obsolete catalog cleanup.
- Centralized release information and supported a separate documentation URL.

### Release checks still pending

The developer’s active Codex App is not used for testing, so there is no real App end-to-end validation result. Windows hardware validation, signing, notarization and public installers remain release-stage work.
