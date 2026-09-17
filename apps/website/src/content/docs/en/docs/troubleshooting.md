---
title: Troubleshoot model connections and recovery
description: Resolve API errors, stale Codex model lists, configuration conflicts, session cache failures and remote compaction issues in Codex Switch.
---

## Connection test fails

| Symptom                        | Check                                                                         |
| ------------------------------ | ----------------------------------------------------------------------------- |
| 401 / 403                      | Key validity and whether it belongs to this plan or endpoint                  |
| 404                            | API URL, path prefix and model ID                                             |
| 429                            | Provider credits and rate limits                                              |
| 400 / 422                      | Model support for the API format and parameters                               |
| Timeout                        | Local network and service availability                                        |
| Response without complete text | Reasoning-only output, insufficient budget or an incompatible response format |

The app does not display complete upstream error bodies because they may contain keys or request data. Check the provider console first.

## Codex did not change after activation

Codex may not have reloaded its settings. Finish the task, quit normally and reopen through Codex Switch. Existing task labels may retain old names even though later requests through the local router use the current configuration. Reopening may be needed to refresh the model list.

## Another program modified the configuration

Stop other programs from writing the configuration and retry. Disabling Codex Switch automatically saves a copy of later edits and restores the activation baseline without another confirmation. If saving the copy or restoring fails, the error is shown and the service remains enabled. See [recovery rules](/en/docs/switch/).

## Local port is busy

Activation stops without overwriting the configuration. Quit the known program using that port and retry. Do not terminate unfamiliar processes blindly.

## Cannot delete a connection

Adding, editing and deleting are disabled during activation. Turn off Codex Switch first. At least one configuration must be selected before enabling again.

## No connection after a crash

Reopen Codex Switch, click **恢复连接 (Restore connection)** and reopen Codex as prompted, or disable the switch to restore the original settings. Session caches do not survive a process restart; old references may require a new task.

## Session cache expired after switching

Requests now use the current configuration and do not fall back to the old service. Context is cached in memory for up to 64 responses and one hour, subject to size limits. The cache does not survive exit or restart. If the client sends only an old response reference without enough context to reconstruct it, the router reports an error; provide full context or start a new task. Provider-specific context formats may also be incompatible.

## Model settings differ from outer settings

A model can override the API URL, format and full-URL mode. Open **编辑 (Edit)** for that model or use **继承外层设置 (Inherit outer settings)** to clear the overrides, then confirm and save. Manual tests use these effective settings too. See [field reference](/en/docs/configuration/).

## Compaction fails or its toggle cannot be changed

Remote compaction requires upstream support; keeping it off is recommended. Ordinary summary requests do not need it. Disable the home screen switch before editing the preference. It saves automatically and applies at the next activation. See [settings](/en/docs/settings/).

## Report an issue

Include your operating system, version, service name, API format and a redacted error in [GitHub Issues](https://github.com/goo-yyh/codex-switch/issues). Do not upload `.env`, complete configurations, API keys or private task content.
