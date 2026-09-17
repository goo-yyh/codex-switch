---
title: Configuration and model capabilities
description: Configure API URLs, keys, default models, context windows, reasoning levels, image input and per-model connection overrides in Codex Switch.
---

A configuration stores one API key and 1–20 models. Each model inherits the outer connection settings unless you override its URL, API format or capabilities.

## Connection fields

| Field                   | Purpose                                                                                                                             |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Provider / service plan | Sets initial URL, API format, models and capabilities. Changing plans resets these initial values and clears the key being entered. |
| Configuration name      | Identifies the card and prefixes model names in Codex. Must be unique, ignoring case and surrounding whitespace.                    |
| API URL                 | The service’s base URL. Path prefixes are preserved. See [URL examples](/en/docs/relay/#which-url-to-enter).                        |
| Full URL                | Sends ordinary requests directly to the entered URL without appending an API path.                                                  |
| API format              | Select Responses or Chat Completions to match the service.                                                                          |
| API key                 | Shared by all models in the configuration and stored in the system credential store. Different keys need separate configurations.   |

You can save several configurations for the same provider. Default names increment, such as `Kimi` and `Kimi-2`, and can be edited. An existing configuration’s provider cannot be changed; create a new one instead.

When editing, leave the key blank to retain it. Changing a request URL, including a model-specific URL, or full-URL mode requires re-entering the corresponding key. Do not put credentials in the URL field.

## Models and the default

The first selected model is the default. Use **设为默认 (Set as default)** beside another selected model to change it. Removing the default promotes the first remaining model. For custom models, enter the provider’s exact model ID, not a display name.

**已配置 (Already configured)** means another configuration uses that model with the same provider, URL and API format. It does not prevent reuse or remove other configurations.

## Per-model editing

Select a model and click **编辑 (Edit)**. Changes apply only to that model in the current configuration. **After confirming the dialog, also save the main configuration form.**

[![Model editor with connection overrides, context size, reasoning, images and parallel tools](/screenshots/model-options.png)](/screenshots/model-options.png)

_Actual Chinese interface in the browser preview, using an example configuration._

| Setting                         | Meaning                                                                                                                                                           |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| API URL / API format / full URL | Inherits the outer settings unless overridden. **继承外层设置 (Inherit outer settings)** clears these three overrides. An empty URL still inherits the outer URL. |
| Context length                  | Declares the context capacity to Codex, from 4,096 to 2,000,000 tokens. A larger value does not increase the upstream model’s actual capacity.                    |
| Reasoning levels                | Comma-separated supported values, such as `low,high,max`. Only list values the service supports.                                                                  |
| Default reasoning level         | Must be included in the supported reasoning levels.                                                                                                               |
| Image input                     | Use the model registry or explicitly declare text-only or text-and-image support. This does not add vision to a text-only model.                                  |
| Parallel tool calls             | Use the default or explicitly declare support. Actual behavior depends on the service and model.                                                                  |

Accepted reasoning level names are `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` and `ultra`. Not every model supports every level. Unset capabilities use presets or model catalog defaults.

For example, a configuration may use Responses while one model overrides its format to Chat Completions and optionally uses a different URL. Both models still share one key. Split them into separate configurations if they require different credentials.

## Save, test and apply

### Context headroom

Built-in models use **80% of the documented capacity, rounded down**, leaving room for requests and output. Examples: 1,000,000 → 800,000, 262,144 → 209,715, and 204,800 → 163,840. Each model has its own value instead of inheriting the vendor flagship's larger window. GLM's 1M examples differ; the preset conservatively uses a 1,000,000-token base.

Kimi Coding's `k3` defaults to 209,715 based on Moderato's 256K entitlement; Allegretto and higher members can use 838,860. `k3-256k` stays at 209,715. `kimi-for-coding` now maps to K2.8 Preview and defaults to 838,860. Source: [official plan specifications](https://www.kimi.com/code/docs/kimi-code/models.html).

New configurations and plan selections load these presets. Existing context values remain unchanged and can be adjusted in the model editor. Saving and enabling do not apply the 80% factor again. Codex still decides when to compact; this headroom reduces overflow risk but does not guarantee that every request fits.

### Actions

- **Save configuration:** stores changes without testing, selecting or enabling it.
- **Test configuration:** checks basic text responses using each model’s effective URL and format; does not save or enable. Testing can be cancelled; fields and navigation are locked while it runs.
- **Enable Codex Switch:** applies models from selected configurations to Codex. Disable before editing, save, then enable again.

Selecting multiple configurations makes all their models available. Failed requests do not automatically try another configuration. See [switching and recovery](/en/docs/switch/) for existing-session behavior.

## Reasoning defaults

The model editor loads reasoning settings for the selected model and plan, including missing fields in older configurations. Explicit saved values remain unchanged. Defaults follow official documentation rather than a universal high setting: GLM 5.3/5.2 and the ordinary Kimi K3 API use max; Kimi Coding's k3/k3-256k use high and kimi-for-coding uses max. Qwen 3.8 uses xhigh; documented Qwen 3.7/3.6 entries use medium; DeepSeek uses high.

MiniMax M3 defaults to none in the ordinary Responses API, while its official Codex plan example uses high. For models with only a thinking toggle, the editor explains that none/high represent off/on rather than native reasoning intensity levels. Models without a separately documented default retain automatic selection.

This review used documentation, without live API-key testing. Sources: [Qwen](https://help.aliyun.com/zh/model-studio/codex), [GLM](https://docs.bigmodel.cn/cn/guide/capabilities/thinking), [MiniMax](https://platform.minimax.cn/docs/api-reference/responses-create), [Kimi](https://platform.kimi.com/docs/api/chat), [DeepSeek](https://api-docs.deepseek.com/guides/thinking_mode/).
