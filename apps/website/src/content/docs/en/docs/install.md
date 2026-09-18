---
title: Installation and downloads
description: Check Codex Switch installer availability and learn how to install, open, update and uninstall the app on macOS and Windows.
---

## Download the app

Codex Switch installers will be published on [GitHub Releases](https://github.com/goo-yyh/codex-switch/releases). **No public installers have been released yet.** Once available, choose the package for your system and check that version’s release notes for supported platforms.

| System  | Choose your package                                                                                               |
| ------- | ----------------------------------------------------------------------------------------------------------------- |
| macOS   | Open **About This Mac** in the Apple menu and check the chip. Choose the matching Apple Silicon or Intel package. |
| Windows | Open **Settings → System → About** and check the system type. Choose an installer for that architecture.          |

See the [changelog](/en/docs/changelog/) for updates. The steps below apply once installers are available.

## Install the app

### macOS

1. Download the `.dmg` package matching your Mac’s chip.
2. Open it and drag Codex Switch into **Applications**.
3. Open Codex Switch from **Applications**.

### Windows

1. Download the installer matching your system.
2. Run it and follow the installation wizard.
3. Open Codex Switch from the Start menu.

## First launch

Have [Codex App](https://chatgpt.com/download/) installed, plus a model provider API key and available credits.

On first launch, the empty workspace looks like this. Click **新增配置 (Add configuration)** in the center, then follow the [three-step quickstart](/en/docs/quickstart/). After saving, select the configuration and enable the service manually.

[![First launch: Add configuration in the center and the disabled service switch below](/screenshots/first-launch.png)](/screenshots/first-launch.png)

_Captured from the current app UI in browser preview with example data; connection and recovery states are simulated. Click an image for the original._

The app starts in Chinese by default. Use the language button to the left of **Docs** to switch to English. Your choice is remembered the next time you open the app.

## Update the app

The app checks for stable releases in the background. When a newer version is available, a blue download icon and version number appear beside the language button. Click to download the matching installer directly through your default browser, without visiting the release page. Installation and restarting remain manual.

Before installing, disable the service and quit Codex and Codex Switch normally, then follow the installation steps above. Existing configurations stay on your device, so you do not need to recreate them.

## Uninstall the app

Disable the service to restore your original configuration, then quit Codex and Codex Switch normally. Remove the app from **Applications** on macOS or **Installed apps** on Windows.

Deleting the app alone does not restore configuration or remove credentials and local app data.
