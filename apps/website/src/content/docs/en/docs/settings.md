---
title: Compaction, startup and recovery settings
description: Manage remote context compaction, persistent startup preferences, Codex App detection and recovery of your original configuration.
---

Open **设置 (Settings)** at the top right. This page manages global preferences. Edit individual configurations for connection URLs and model parameters.

[![Settings for remote compaction, startup and configuration recovery](/screenshots/settings.png)](/screenshots/settings.png)

_Actual Chinese interface in the browser preview. App detection and configuration paths are preview data._

## Remote context compaction

**Off by default; keeping it off is recommended.** Server-side compaction requires the upstream service to support the corresponding protocol.

| State | Behavior                                                                                                                                                                                    |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Off   | Does not enable remote compaction flags. Codex can still generate text summaries through ordinary model requests. This does not disable all context compaction.                             |
| On    | Enables remote compaction flags. Codex chooses the remote protocol and the local gateway forwards supported requests. Unsupported upstreams may fail; automatic fallback is not guaranteed. |

Changes **save automatically** and persist after reopening, whether on or off. Disable the main Codex Switch toggle before editing. The preference applies the next time you enable the connection; an already running Codex may need to reload its configuration.

This preference applies to all selected configurations, not individual models. Ordinary text summaries and remote compaction are different capabilities. The [summary smoke test](/en/docs/compatibility/#ordinary-summary-check) only establishes that the specified models returned summaries.

## Start at login

Off by default. When enabled, Codex Switch starts when you log in to the computer. This preference is saved separately from remote compaction.

Closing the window keeps the tray and background connections running. To fully quit, use **完全退出 Codex Switch (Quit Codex Switch)** in settings or the tray menu. Finish your tasks and quit Codex normally first. See [switching and recovery](/en/docs/switch/).

## Codex App detection and recovery

The **Codex App** section shows the detected app and running state. Use **重新检测 (Detect again)** after installing or moving it.

**配置位置 (Configuration location)** shows the active `config.toml` path, usually `~/.codex/config.toml`; when `CODEX_HOME` is set, use the displayed path. **恢复开启前的配置 (Restore previous configuration)** restores the saved backup. Normally, turning off the main switch is sufficient. If an external modification conflicts, preserve the current version before following the recovery prompt.

The app changes the configuration file, not login or session files. See [switching and recovery](/en/docs/switch/) for backup, crash recovery and existing-session behavior.
