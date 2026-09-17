---
title: Local storage, privacy and API keys
description: Learn where Codex Switch stores model settings and API keys, how requests reach providers, and what deleting a configuration does and does not remove.
---

## Local storage

- Configuration names, models, URLs and preferences are stored in local SQLite.
- API keys are stored in the system credential store, never returned for frontend display or written into the model catalog.
- Recovery records and local access tokens live in the app’s data directory and the projected configuration. macOS file permissions restrict access to the current user.
- The first version has no telemetry, cloud account or centralized request log.

## Network requests

Saving a configuration sends no test requests. Tests run only when you click **测试配置 (Test configuration)**. During ordinary use, prompts and tool-related content are sent to your selected provider. The local router adds provider credentials; client headers are not forwarded unchanged.

The router binds only to `127.0.0.1` and requires a random access token. Public HTTPS certificate verification is enabled and upstream redirects are disabled.

## Development credentials

The repository’s `.env` is read only by explicitly invoked provider test commands. The desktop frontend, website and release bundle do not load it. The repository includes only an empty `.env.example`.

Do not commit real keys, configuration backups or complete request content. Run `pnpm check:secrets` before committing; it reports affected file paths without printing credentials.

## Delete local data

Disable Codex Switch before deleting a configuration. Deletion removes its card and selection; the next activation includes only current configurations. Old session identifiers route to the current configuration, rather than reading historical keys or calling old endpoints.

Historical revisions in the local database and system credential store are not automatically purged yet. **Deleting a configuration does not completely erase its key.** To remove all local data, first disable the switch and quit both apps, then manually clear the app’s data directory and its system credential store entries. Confirm that the original Codex configuration has been restored before uninstalling.
