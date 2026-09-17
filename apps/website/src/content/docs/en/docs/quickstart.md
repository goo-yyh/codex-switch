---
title: Connect Codex App in three steps
description: Follow screenshots to add a model provider, choose models, enable Codex Switch and open Codex App with your selected configurations.
---

Have Codex App installed and your provider’s API key, API URL and model IDs ready. See [installation](/en/docs/install/) for building the app.

> Screenshots show the actual Chinese interface in the browser preview, using example configurations and a fictional key. Connection, backup and process states are simulated, not evidence of native app or provider connectivity. Click an image to view the original.

## 1. Add a configuration

Click **新增配置 (Add configuration)** and choose a provider. For Kimi Coding or Qwen Token Plan, also choose the matching **服务套餐 (Service plan)**. Choose **coding plan** for a custom service. Turn off the main Codex Switch toggle before adding, editing or deleting configurations.

Enter a recognizable name, check the API URL and format, then scroll down to enter the corresponding API key. Preset URLs are usually a starting point; see [plans and custom APIs](/en/docs/relay/) for complete endpoint URLs.

[![Add a Kimi configuration, name it and check its URL and Responses format](/screenshots/connection.png)](/screenshots/connection.png)

## 2. Select models and save

Select 1–20 models. Add an exact model ID if it is not listed. Use **设为默认 (Set as default)** to change the default. **编辑 (Edit)** beside a model opens its URL and capability settings; see [configuration and capabilities](/en/docs/configuration/).

[![Two selected Kimi models, default model and save controls](/screenshots/models.png)](/screenshots/models.png)

Click **保存配置 (Save configuration)** to return home. **Saving sends no request and does not enable the configuration.** **测试配置 (Test configuration)** requests basic text from each selected model but does not save. Real provider tests may incur API charges.

## 3. Enable and open Codex

Select one or more configurations on the home screen, then enable the bottom **Codex Switch** toggle. The native app first backs up the original configuration, then writes the selected models into Codex’s model catalog.

[![Select Kimi and DeepSeek configurations before enabling](/screenshots/selected.png)](/screenshots/selected.png)

Once the status is **已开启 (Enabled)**, click **打开 Codex (Open Codex)**. If Codex is already running and needs to reload its configuration, finish the current task before restarting normally. See [launch and restart](/en/docs/launch/).

[![Enabled configurations and Open Codex button with simulated browser-preview status](/screenshots/enabled.png)](/screenshots/enabled.png)

Models appear as `configuration-name-model`. For example, the pictured configuration offers `Kimi 示例-kimi-k3` and `Kimi 示例-kimi-k2.7-code`. Initial activation uses the default model of the first selected configuration.

## Next steps

- Adjust connections and model parameters: [Configuration and capabilities](/en/docs/configuration/).
- Manage compaction and startup: [Settings](/en/docs/settings/).
- Switch services, handle existing sessions or restore files: [Switching and recovery](/en/docs/switch/).
- Diagnose failures: [Troubleshooting](/en/docs/troubleshooting/) and [compatibility](/en/docs/compatibility/).
