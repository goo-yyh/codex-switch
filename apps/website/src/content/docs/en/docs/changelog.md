---
title: Changelog
description: Follow Codex Switch updates to language switching, model management, automatic recovery, tray controls and documentation.
---

Published versions and assets are available on [GitHub Releases](https://github.com/goo-yyh/codex-switch/releases).

## v0.2.0 · September 20, 2026

[Release notes](https://github.com/goo-yyh/codex-switch/releases/tag/v0.2.0) · [Download and install](/en/docs/install/)

- **Service controls:** enabling the service keeps the bottom switch and Open Codex button available while an overlay locks the rest of the interface. Restarting still requires confirmation; shutdown errors appear in the control bar for retry.
- **Model registry v2:** bundles 21 standard candidates, adding Qwen `qwen3.8-omni-flash` and GLM `glm-5.3-flashx`, with two DeepSeek candidates. Existing configurations, parameters and selected models are preserved.
- **Registry sync:** checks the website catalog on startup and merges new models only when its version is higher. Failures retain the local catalog.
- **Installers:** macOS Apple Silicon, macOS Intel and Windows x64 packages with SHA-256 checksums. Installers remain unsigned.

## v0.1.0 · September 18, 2026

[Release notes](https://github.com/goo-yyh/codex-switch/releases/tag/v0.1.0) · [Download and install](/en/docs/install/)

- **Public installers:** macOS Apple Silicon, macOS Intel and Windows x64, with SHA-256 checksums. Installers are unsigned; macOS apps are not notarized.
- **Update notifications:** stable release checks offer a download; installation remains manual.

- **Languages:** switch app and tray menus between Chinese and English; your choice persists after reopening.
- **Models:** use multiple models, configurations and plan presets. Model lists expand in place with per-model capability settings.
- **Service controls:** settings lock while enabled; disabling restores the original configuration. Saving or selecting does not enable the service.
- **Tray controls:** select multiple configurations without closing the menu, toggle the service, open the panel or quit.
- **Compaction:** preferences save automatically and remote compaction defaults to off. The fallback queue and separate route-save action have been removed.
- **Guides:** bilingual screenshot walkthroughs, fewer duplicate pages and clearer connection and recovery steps.

New here? Read the [three-step quickstart](/en/docs/quickstart/). Check [installation](/en/docs/install/) for download availability.
