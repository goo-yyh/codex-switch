---
title: Switch providers and restore configuration
description: Enable multiple Codex model configurations, switch from the tray, restore original settings and handle external edits or crash recovery.
---

The main switch is in the bottom connection controls. Select the configuration cards you want to expose to Codex, then enable it. The card area scrolls independently while the switch and launch button remain visible.

## Tray shortcuts

After closing the main window, use the menu bar or system tray icon:

- **Enable selected configurations:** applies the home screen selection, including multiple providers.
- **Disable and restore:** restores the settings from before this activation, just like the home screen switch.
- **Switch and enable one configuration:** applies all models in the chosen saved configuration and synchronizes the home screen selection. It also enables the connection if needed. To combine configurations, use the home screen.
- **Restore background connection:** rebuilds routing for the current selection. Editing is disabled during activation; selection changes apply immediately.
- **Quit Codex Switch:** available by right-clicking the tray icon. Restores settings before quitting and asks you to quit Codex first if it still uses the local connection.

A check mark indicates the applied configuration revision, not an unapplied selection. If no configurations exist, open the main window to add one.

A failed switch keeps the previous state and opens the main window with the reason. Recovery conflicts require handling in the main window; the tray does not force overwrites. After switching, subsequent requests from existing sessions also use the current configuration. Reopen Codex to refresh a cached model list or restored settings. The app does not automatically restart Codex or end tasks.

## Enable

1. Read the current `config.toml`, preserving both its bytes and whether it existed.
2. Write the recovery record to Codex Switch’s data directory first.
3. Atomically write the local routing and model catalog settings.

At least one configuration must be selected. Selection changes apply immediately while enabled, and the last selected configuration cannot be deselected. Disable before adding, editing or deleting configurations. Re-enabling or switching does not overwrite the earliest backup from this activation.

## Disable

Disabling restores the file from **before this activation**, including comments and line endings. If `config.toml` did not exist, the generated file is removed. The next activation takes a fresh backup.

Codex Switch does not change `auth.json`, sessions or history. Recovery restores `config.toml`, not a snapshot of the entire Codex directory.

## Codex is still running

Codex may retain the old settings in memory after file recovery. The app prompts you to reopen it and temporarily keeps local routing alive to avoid abruptly disconnecting current requests. Quit Codex normally, then reopen it to use the restored settings.

## Another program edited the file

External edits are not silently overwritten. Keep the current file, or choose **Back up and restore**. The latter saves a separate copy of the current file in the app data directory before restoring the original backup.

## Unexpected app exit

The recovery record is written before Codex’s configuration is changed. Reopen Codex Switch and use **恢复连接 (Restore connection)** to rebuild local routing without opening or restarting Codex, or turn the switch off to restore the original file. A running Codex may need a normal restart to read the new connection. Routing is unavailable while Codex Switch is stopped; this version does not secretly restart either app.

## Backup location

The app data directory holds `activation.json`. Conflict backups are named `before-restore-<number>.toml`. On macOS the directory is normally `~/Library/Application Support/com.codexdocs.switch/`; Windows uses the system application data directory.

Backups may contain sensitive fields already present in your configuration. Keep them local and do not attach complete backups to issue reports.

## Failed switches and catalog cleanup

A failed activation or switch restores the pre-operation configuration, retaining existing routing and the default selection. The earliest activation backup remains the recovery baseline. If another program edits the file during the operation, that change is preserved and a conflict is reported.

When Codex is not running and no requests are active, unused generated model catalogs are removed unless referenced by current settings or the recovery baseline. Unavailable keys in selected configurations cause the application attempt to fail without replacing the previous state. Unselected configurations are omitted from the new catalog.

Background recovery uses the current selection. Old model identifiers are retained only for existing-session compatibility, not historical credentials or endpoints: if the same configuration still has the original model, its latest revision is used; otherwise the current default model is used.
