---
title: 三步连接
description: 按真实界面截图新增配置、选择模型并开启 Codex Switch。
---

准备好已安装的 Codex App，以及服务商提供的 API Key、API 地址和可用模型。应用安装方式见[安装与下载](/docs/install/)。

> 以下为当前应用的真实界面截图，采集于浏览器预览，使用示例配置和虚构 Key。预览中的连接、备份与进程状态为模拟数据，不代表原生端或服务连通验证。点击截图可查看原图。

## 1. 新增配置

在首页点击「新增配置」，选择厂商；使用 Kimi Coding 或千问 Token Plan 时，再选择对应的「服务套餐」。自定义服务选择「coding plan」。新增、编辑和删除前需先关闭 Codex Switch 总开关。

填写便于识别的配置名称，核对 API 地址与接口格式，向下滚动填写该服务的 API Key。通常可保留预设地址；完整端点的填写方式见[套餐与自定义接口](/docs/relay/)。

[![新增配置：选择 Kimi、填写名称、检查地址与 Responses 接口](/screenshots/connection.png)](/screenshots/connection.png)

## 2. 选择模型并保存

勾选 1–20 个模型。列表没有的模型可输入准确 ID 后添加。点击「设为默认」调整默认模型；点击模型旁的「编辑」设置地址或能力，详见[配置字段与模型能力](/docs/configuration/)。

[![选择两个 Kimi 模型，默认模型与底部保存入口](/screenshots/models.png)](/screenshots/models.png)

点击「保存配置」回到首页。**保存不会发送请求，也不会自动启用配置。**「测试配置」会逐个请求当前所选模型，仅检查基础文本连通；它不会替你保存。测试使用真实服务时可能产生 API 费用。

## 3. 勾选配置并开启

首页可同时勾选多个配置，至少选中一个后，打开底部「Codex Switch」总开关。原生应用会先备份原配置，再把所选配置的模型写入 Codex 模型目录。

[![首页同时选择 Kimi 和 DeepSeek 配置，准备开启](/screenshots/selected.png)](/screenshots/selected.png)

显示「已开启」后点击「打开 Codex」。如果 Codex 已在运行且需要重新加载配置，先完成当前任务，再按提示正常重启。详见[打开与重启 Codex](/docs/launch/)。

[![开启后的配置列表与打开 Codex 按钮，浏览器预览中的模拟状态](/screenshots/enabled.png)](/screenshots/enabled.png)

模型以 `配置名称-模型` 显示。例如，截图中的 Kimi 配置会提供 `Kimi 示例-kimi-k3` 和 `Kimi 示例-kimi-k2.7-code`。首次应用使用第一个选中配置的默认模型。

## 接下来

- 调整连接与模型参数：[配置字段与模型能力](/docs/configuration/)。
- 管理压缩和启动开关：[通用设置](/docs/settings/)。
- 切换服务、处理旧会话或恢复文件：[开启、关闭与恢复](/docs/switch/)。
- 排查连接失败：[常见问题](/docs/troubleshooting/)与[兼容范围](/docs/compatibility/)。
