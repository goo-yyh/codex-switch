---
title: Switch providers and restore configuration
description: Enable multiple Codex model configurations, switch from the tray, restore original settings and handle external edits or crash recovery.
---

The main switch is in the bottom connection controls. Select the configuration cards you want to expose to Codex, then enable it. The card area scrolls independently. While enabled, an overlay locks the interface and offers only **关闭服务 (Disable service)**.

## Tray shortcuts

After closing the main window, use the menu bar or system tray icon:

- **Enable Codex Switch:** applies the home screen selection, including multiple providers.
- **Disable and restore:** restores the settings from before this activation, just like **Disable service** in the overlay.
- **Select configurations (multiple):** select or deselect saved configurations in sync with the home screen. While disabled, this only saves your selection; click **Enable Codex Switch** to start the service. The selection submenu and all its items are disabled while the service is enabled.
- **Restore background connection:** rebuilds routing for the current selection. Recovery uses the current configuration without changing selections or settings.
- **Quit app:** available by right-clicking the tray icon. Restores settings before quitting and asks you to quit Codex first if it still uses the local connection.

On macOS, clicking a configuration checkbox keeps the menu open for further selections. Each change is saved immediately without enabling the service.

Check marks show the current selection and remain visible while the service is disabled. If no configurations exist, open the main window to add one.

A failed switch keeps the previous state and opens the main window with the reason. Disabling automatically restores the original configuration without another confirmation. External edits are saved to a separate copy first. After switching, subsequent requests from existing sessions also use the current configuration. Reopen Codex to refresh a cached model list or restored settings. The app does not automatically restart Codex or end tasks.

## Enable

1. Read the current `config.toml`, preserving both its bytes and whether it existed.
2. Write the recovery record to Codex Switch’s data directory first.
3. Atomically write the local routing and model catalog settings.

At least one configuration must be selected. While enabled, a non-dismissible overlay offers only **Disable service**. Disable before changing selections, adding, editing or deleting configurations, or changing general settings. Tray selection is disabled too. Re-enabling or switching does not overwrite the earliest backup from this activation.

## Disable

Disabling restores the file from **before this activation**, including comments and line endings. If `config.toml` did not exist, the generated file is removed. The next activation takes a fresh backup.

Codex Switch does not change `auth.json`, sessions or history. Recovery restores `config.toml`, not a snapshot of the entire Codex directory.

## Codex is still running

Codex may retain the old settings in memory after file recovery. The app prompts you to reopen it and temporarily keeps local routing alive to avoid abruptly disconnecting current requests. Quit Codex normally, then reopen it to use the restored settings.

## Another program edited the file

Disabling automatically restores the configuration from before activation without another confirmation. If another program edited the file, a separate copy is saved in the app data directory first. If that copy cannot be saved, the current file is preserved and an error is shown.

## Unexpected app exit

The recovery record is written before Codex’s configuration is changed. Reopen Codex Switch and use **恢复后台连接 (Restore background connection)** in the tray to rebuild local routing without opening or restarting Codex, or click **Disable service** in the overlay to restore the original file. A running Codex may need a normal restart to read the new connection. Routing is unavailable while Codex Switch is stopped; this version does not secretly restart either app.

## Backup location

The app data directory holds `activation.json`. Conflict backups are named `before-restore-<number>.toml`. On macOS the directory is normally `~/Library/Application Support/com.codexdocs.switch/`; Windows uses the system application data directory.

Backups may contain sensitive fields already present in your configuration. Keep them local and do not attach complete backups to issue reports.

## Failed switches and catalog cleanup

A failed activation or switch restores the pre-operation configuration, retaining existing routing and the default selection. The earliest activation backup remains the recovery baseline. If another program edits the file during the operation, that change is preserved and a conflict is reported.

When Codex is not running and no requests are active, unused generated model catalogs are removed unless referenced by current settings or the recovery baseline. Unavailable keys in selected configurations cause the application attempt to fail without replacing the previous state. Unselected configurations are omitted from the new catalog.

Background recovery uses the current selection. Old model identifiers are retained only for existing-session compatibility, not historical credentials or endpoints: if the same configuration still has the original model, its latest revision is used; otherwise the current default model is used.
