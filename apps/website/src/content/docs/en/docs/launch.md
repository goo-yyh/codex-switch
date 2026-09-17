---
title: Open and restart Codex App
description: Launch Codex after enabling model configurations, refresh its model list safely and understand tray behavior and normal app shutdown.
---

## Open Codex

Save configurations, select them on the home screen and enable the main switch, then click **打开 Codex (Open Codex)**. The button only launches the app. Selection changes apply immediately while enabled; configuration contents require disabling before editing. macOS detection uses the application identifier rather than just a filename.

## After configuration changes

Subsequent requests through the local router from new and existing sessions use the current configuration. Codex may still display a cached model list; restart normally after finishing the current task to refresh it. Restored settings may also require reopening.

Restart checks for active local routing requests and blocks while any are running. Even with zero active requests, Codex may still be executing local tools, so confirm that the task has finished.

Restart requests a normal app exit. If Codex refuses to quit or times out, the operation stops and asks you to exit manually. It never force-kills the process.

## Close the window or quit fully

Closing the Codex Switch window leaves it in the system tray with connections active. Use the tray to enable or disable routing, select all models from one configuration or reopen the window. See [tray shortcuts](/en/docs/switch/#tray-shortcuts).

To quit fully, right-click the tray icon and choose **退出 Codex Switch (Quit Codex Switch)**, or use the corresponding settings action. If Codex still uses the local connection, quit Codex first when prompted. A normal full exit restores the original configuration; if recovery fails, routing remains active and the reason is shown.

## Codex was not found

Install the official app, then run detection again in settings. Windows paths may vary by distribution; complete testing on Windows hardware remains pending.
