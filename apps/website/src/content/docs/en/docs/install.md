---
title: Install and build Codex Switch
description: Build Codex Switch from source for macOS or Windows, check platform prerequisites, and safely update or uninstall the app.
---

Source builds are currently available; public installers have not been released. The [download page](/en/download/) is the source of release availability.

## Prerequisites

- Install the official [Codex App](https://chatgpt.com/download/).
- Have a provider API key and available credits.
- Source builds require Node.js 22.12+, pnpm 10, stable Rust and platform build tools.

## Build from source

```sh
git clone https://github.com/goo-yyh/codex-switch.git
cd codex-switch
pnpm install --frozen-lockfile
pnpm --filter @codex-switch/desktop tauri build
```

macOS requires Xcode Command Line Tools. Outputs in `target/release/bundle/` include `.app` and `.dmg` files. The command builds for the local architecture; Intel and Apple Silicon need the corresponding environment or an explicit target.

Local preview builds are not notarized by Apple. Public distribution still requires signing, notarization and release validation. Do not disable system-wide security protections to install the app.

## Windows

Microsoft C++ Build Tools and WebView2 are required. Build an NSIS installer with:

```sh
pnpm --filter @codex-switch/desktop tauri build --config src-tauri/tauri.windows.conf.json
```

CI includes Windows build checks. Local macOS validation does not establish compatibility on a real Windows device.

## Update and uninstall

The first version does not update automatically. Quit Codex normally, then fully quit Codex Switch before upgrading. Installing a newer build preserves local connection metadata.

Before uninstalling, turn off the main switch and confirm that the original configuration has been restored, then quit both apps normally. Deleting the app alone does not restore settings. Credentials and app data may remain; remove unneeded connections in the app before uninstalling. See [data deletion limits](/en/docs/privacy/#delete-local-data).
