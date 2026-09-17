---
title: coding plan
description: Codex Switch coding plan与使用说明。
---

在新增配置时选择「coding plan」，填写名称、API Key、一个或多个模型 ID 与 HTTPS API 地址。

## 地址怎么填

例如服务要求请求 `https://api.example.com/prefix/v1/responses`，可以填写 `https://api.example.com/prefix/v1`。应用保留路径前缀，不会自动重复添加 `/v1`。粘贴完整 `/responses` 或 `/chat/completions` 地址时会归一化。

API 地址填写服务商提供的基础地址，应用按接口格式追加请求路径。

地址不能包含账号或片段；基础地址模式也不接受查询参数。Key 应放在单独的密钥字段。第一版不接受 HTTP 上游或本地回环上游。

## 接口格式

**Responses（原生）**：中转站明确支持 OpenAI Responses API 时使用。请求在本地更换模型标识并转发到该站点。

**Chat Completions（自动转换）**：只有 Chat 接口时使用。使用 CC Switch 模块转换文本、声明支持的图片与工具调用，逐条转为 Codex 所需的 Responses 事件。请阅读[兼容范围](/docs/compatibility/)。

两种格式不会自动互相重试，也不会在失败后自动换服务。这样可避免重复计费与请求被发送到意料之外的站点。

## 请求会经过谁

Codex → 本机回环路由 → 你填写的服务。服务商的实际存储与处理政策由其自行负责。Codex Switch 不经营中转站，也不为第三方额度或服务品质背书。

## API 地址与上下文压缩

每个配置只使用一个 API 地址，选择厂商时填入默认地址，也可以直接修改。底部「测试配置」会逐个测试当前地址下的已选模型，测试不会保存或启用配置。更换地址后需填写对应服务的 Key。

通用设置中的「远程上下文压缩」开关会自动保存，重新打开应用后保留上次状态。请先关闭 Codex Switch 再修改，下次开启时生效；已运行的 Codex 可能需要重新加载配置。
