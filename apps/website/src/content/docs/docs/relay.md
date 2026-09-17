---
title: Coding Plan 套餐与自定义 API
description: 为 Codex App 连接 Coding Plan 或自定义 HTTPS 服务，正确选择 Responses 与 Chat Completions，填写基础地址或完整请求端点。
---

内置厂商可在「服务套餐」中选择专用套餐。没有对应预设的服务，在新增配置时选择「coding plan」，手动填写名称、地址、接口格式、Key 与模型 ID。

## 按量 API 与套餐

按量 API、Coding Plan、海外节点和企业网关可能使用不同密钥、地址与模型。请将服务商提供的三项信息配套填写，不因 Key 格式相似就认为可以混用。

当前内置的额外套餐如下，属于本项目的初始配置值：

| 套餐            | API 基础地址                                                         |
| --------------- | -------------------------------------------------------------------- |
| Kimi Coding     | `https://api.kimi.com/coding/v1`                                     |
| 千问 Token Plan | `https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1` |

选择套餐会载入对应地址、模型和能力，并清空待填写的 Key。实际账户权限以服务商为准；修改预设不会覆盖已保存的配置。

## 地址怎么填

| 服务商提供的地址   | 填写方式                                                                                      | 普通请求实际地址                              |
| ------------------ | --------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Responses 基础地址 | 填 `https://api.example.com/prefix/v1`，关闭「完整 URL」，选择 Responses                      | `https://api.example.com/prefix/v1/responses` |
| Chat 基础地址      | 填 `https://api.example.com/v1`，关闭「完整 URL」，选择 Chat Completions                      | `https://api.example.com/v1/chat/completions` |
| 完整请求端点       | 填 `https://api.example.com/gateway/generate?version=1`，开启「完整 URL」，选择其实际接口格式 | 原样使用填写的 URL                            |

基础地址模式保留路径前缀，不会重复添加 `/v1`；粘贴已知 `/responses` 或 `/chat/completions` 后缀时会归一化。完整 URL 模式保留路径和查询参数，不追加普通请求路径。

只接受外部 HTTPS 上游，不接受 HTTP 或本地回环地址。地址不能带用户名、密码或 `#` 片段；基础地址模式也不接受查询参数。API Key 填在单独的密钥字段中。

完整 URL 下，旧版远程 compact 路径只能从已知 `/responses` 结尾推导；任意自定义端点不一定可用。普通请求能通不代表远程压缩能通，建议保持[远程上下文压缩](/docs/settings/#远程上下文压缩)关闭。

## 接口格式怎么选

- **Responses**：服务明确提供 Responses API 时选择。本地网关替换模型标识并转发请求。
- **Chat Completions**：服务只有 Chat 接口时选择。复用 CC Switch 的协议转换模块，把文本、图片与工具调用转换为 Codex 所需的 Responses 事件，具体见[兼容范围](/docs/compatibility/)。

失败后不会自动改协议、换域名或尝试备用配置。401 / 403 优先核对密钥和权限，404 核对套餐、模型 ID 与路径。

## 逐模型覆盖与测试

模型默认继承外层地址和接口，也可在模型行的「编辑」中单独覆盖。手动「测试配置」与实际路由都使用各模型最终生效的设置；更换请求地址或完整 URL 模式需重新填写 Key。详见[配置字段与模型能力](/docs/configuration/)。

请求路径为 **Codex → 本机回环路由 → 你配置的服务**。Codex Switch 不经营云端中转；服务商对请求数据的处理取决于其自身政策。
