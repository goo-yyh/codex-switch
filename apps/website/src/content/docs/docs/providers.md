---
title: 连接模型服务
description: 选择 Codex Switch 内置厂商、Coding Plan 套餐或自定义 API，正确填写服务地址、接口格式与密钥。
---

**先选对厂商和套餐，再填对应的 Key。** 按量 API 与订阅套餐可能使用不同的密钥、地址和模型。

## 内置厂商与套餐

点击「新增配置」，选择智谱 GLM、DeepSeek、Kimi、千问或 MiniMax，应用会填入地址和模型预设。

[![新增配置中的五家厂商与自定义服务入口](/screenshots/providers.png)](/screenshots/providers.png)

_截图为应用浏览器预览，使用示例数据，连接状态为模拟。点击图片可放大。_

使用智谱 Coding Plan、MiniMax Token Plan、Kimi Coding 或千问 Token Plan 时，在「服务套餐」中选择对应项，再填写套餐 Key。切换套餐会重新载入预设并清空待填写的 Key。

[![服务套餐选择：智谱 Coding Plan](/screenshots/coding-plan.png)](/screenshots/coding-plan.png)

预设不代表账号拥有模型权限。模型是否可用，以你的套餐为准；也可手动添加服务商提供的模型 ID。

## 自定义 API

没有对应预设时，选择「coding plan」，填写名称、HTTPS 地址、接口格式、Key 和模型 ID。

[![自定义服务的完整 URL、接口和模型填写位置](/screenshots/custom-api.png)](/screenshots/custom-api.png)

图中的 `api.example.com` 和 `example-model` 仅为示例，请换成服务商提供的信息。

## 地址怎么填

| 提供的信息         | 填写方式                                                                                                   |
| ------------------ | ---------------------------------------------------------------------------------------------------------- |
| Responses 基础地址 | 填 `https://api.example.com/v1`，选择 Responses，关闭「完整 URL」。请求会追加 `/responses`。               |
| Chat 基础地址      | 填 `https://api.example.com/v1`，选择 Chat Completions，关闭「完整 URL」。请求会追加 `/chat/completions`。 |
| 完整请求端点       | 原样填写，开启「完整 URL」，选择服务支持的接口格式，不再追加请求路径。                                     |

只接受外部 HTTPS 地址。Key 填在单独的密钥字段，不放在 URL 中。接口格式以服务商说明为准，失败后不会自动换协议或切换服务。

## 保存与检查

- 一个配置共用一个 Key，需要不同 Key 时分别新增配置。
- 编辑时 Key 留空可保留原密钥；更改地址或「完整 URL」模式后需重新填写。
- 「测试配置」检查所选模型的基础文本响应，可能产生费用，不会保存或启用；通过不代表所有模型能力都可用。
- 遇到 401 / 403，检查 Key 和套餐权限；遇到 404，检查地址、接口和模型 ID。

模型能力的调整方式见[模型设置](/docs/configuration/)。
