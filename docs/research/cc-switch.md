# CC Switch 源码研究

研究日期：2026-09-15。本文区分源码事实、本机观察和本项目设计，源码存在不等于实机兼容已验证。

## 1. 参考基线

- 上游：[farion1231/cc-switch](https://github.com/farion1231/cc-switch)。
- 固定提交：`06082e189d65e6d6dbadc35dacdac1ce6c79d89a`。
- 提交日期：2026-09-15；标题：`feat: add MiniMax Code harness support (#7383)`。
- 只读参考克隆：`/tmp/codex-switch-cc-switch-reference`，未复制到产品源码。
- 许可证：[MIT](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/LICENSE)。若后续移植代码，逐模块保留原版权和许可证，并记录来源、修改和对应测试。本次仅编写方案。

## 2. 关键源码证据

### 配置写入与认证分离

[codex_config.rs:1106](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_config.rs#L1106) 的 `write_codex_live_config_atomic` 明确把路由、模型、provider token 配置与 `auth.json` 登录状态分开；此路径只写配置。文件中还有旧的双文件写入函数，不能看到 `write_codex_live_atomic` 就推断所有切换都重写登录文件。

设计影响：本项目默认保留用户现有登录凭据，真实上游密钥放操作系统凭据库；仅向自己的 provider 写本地网关凭据。恢复仅撤销自己拥有的改动。

### Codex 侧继续使用 Responses

[proxy.rs:3439](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/services/proxy.rs#L3439) 的 `apply_codex_proxy_toml_config_for_provider` 将本地连接设置为 `wire_api = "responses"`。真实上游协议由代理内部决定。

[codex.rs:25](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/providers/codex.rs#L25) 及同文件的 `should_convert_codex_responses_to_chat`、`should_convert_codex_responses_to_anthropic` 区分 Responses、Chat Completions 和 Anthropic Messages。

设计影响：用户选“国内模型/中转站”与底层选“Responses/Chat/Messages”是两个维度，不能把所有国内模型都当成 Chat 协议。

### 模型目录是必要适配层

[codex_config.rs:1561](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_config.rs#L1561) 的 `codex_catalog_model_entry` 写入 slug、展示名、上下文、工具和思考能力；[同文件:2277](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_config.rs#L2277) 的 `codex_model_catalog_from_settings` 根据适配路径选择模板。

[同文件:2321](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_config.rs#L2321) 的 `set_codex_model_catalog_json_field` 处理目录指针所有权；用户自己的目录不能无条件覆盖。

设计影响：模型列表接口返回名称，不等于已经知道上下文长度、图片、推理档位或工具格式。必须有单独的、带版本的能力描述。

### 工具与多轮上下文

[transform_codex_chat.rs:258](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/providers/transform_codex_chat.rs#L258) 的转换器处理 instructions、input、工具声明、工具结果、推理参数等。同文件还存在 custom tool、namespace 与工具搜索映射。

[codex_chat_history.rs:33](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/providers/codex_chat_history.rs#L33) 的 `CodexChatHistoryStore` 为 `previous_response_id + function_call_output` 恢复需要的原始调用及 reasoning 信息。

设计影响：只转换第一轮文本会得到“测试成功、实际编码失败”的产品。缓存必须按路由版本和会话隔离；不能只按工具名称匹配，也不能把缓存当成所有会话恢复问题的通用解法。

### 流式响应

[streaming_codex_chat.rs:803](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/providers/streaming_codex_chat.rs#L803) 包含 Chat 流到 Responses 事件流的转换，处理分块 UTF-8、工具参数增量、用量、结束原因和中途失败，后续附有大量边界测试。

设计影响：优先参考转换内核和边界用例；迁移后重新审查错误策略，不照搬“忽略无法解析的分块”等宽松处理。截断、无效工具参数不能伪装成成功完成。

### 路由端点

[server.rs:323](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/server.rs#L323) 注册模型、Responses、压缩等端点；[handlers.rs:960](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/handlers.rs#L960) 包含 compact 处理。

设计影响：首版最少覆盖模型发现、Responses 流式/非流式、取消和压缩能力的明确策略；看到 `/responses` 返回 200 不足以证明 Codex 工作流可用。

### 供应商预设

[codexProviderPresets.ts](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src/config/codexProviderPresets.ts) 已包含国内供应商、套餐端点和模型能力；Kimi、DeepSeek、GLM 等条目使用原生 Responses 预设。

设计影响：可借鉴字段结构及问题线索，不能原样搬运全部预设、推广链接、推荐排序或“已测试”注释。它们只是上游记录，必须用供应商官方资料和本项目测试重新确认。

### 不移植的历史迁移

[codex_history_migration.rs](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_history_migration.rs) 会迁移本机历史 JSONL 和状态数据库中的 provider 归属。

设计影响：本项目不改写用户会话数据库、不冒充内置 `openai` provider、不把历史迁移作为正常配置前置步骤。

## 3. 官方配置依据

使用 OpenAI Docs 工具获取了以下官方正文。研究时部分原 `developers.openai.com/codex/...` 页面已返回指向 `learn.chatgpt.com` 的新文档，不能据旧页面标题推断现版本行为。

- [高级配置](https://developers.openai.com/codex/config-advanced/)：custom provider 的 base URL、认证、HTTP headers；provider/auth 等机器级设置应放用户级配置。文档中的命令式认证需要版本核验。
- [配置参考](https://developers.openai.com/codex/config-reference/)：`wire_api` 当前仅列 `responses`；支持 `model_catalog_json`，说明为启动时加载；`requires_openai_auth` 默认 false；`experimental_bearer_token` 是直接 token 字段，官方不推荐用它存供应商长期密钥。
- [桌面应用](https://developers.openai.com/codex/app/) 与 [设置](https://developers.openai.com/codex/app/settings/)：证明桌面入口与产品范围；不能据此证明第三方 provider 在各版本、各登录状态下都完整可用。

国内供应商官方页面本次抓取有超时；仅从官方搜索摘要确认 DeepSeek 声明 Responses 支持。Kimi/GLM 的具体端点与能力仍主要来自上游源码线索，本项目不据此标记“已验证”。后续核对入口：

- [DeepSeek Codex 接入](https://api-docs.deepseek.com/quick_start/agent_integrations/codex/)
- [DeepSeek Responses](https://api-docs.deepseek.com/guides/responses_api/)
- [GLM Codex 接入](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)
- [Kimi Codex 接入](https://platform.kimi.com/docs/guide/codex-kimi.md)

## 4. 本机应用观察

- 本机应用路径为 `/Applications/ChatGPT.app`。
- `CFBundleIdentifier = com.openai.codex`，`CFBundleShortVersionString = 26.908.40834`。
- 安装包包含 `Resources/codex` 和桌面主进程代码；只读查看包内代码发现 `config/read`、`config/batchWrite`、`reloadUserConfig`、`model/list` 及线程 `modelProvider` 处理。
- 这些内部调用证明应用存在配置/模型/线程层，**不构成允许其他应用调用的公开控制接口**。本项目不依赖私有 IPC、注入桌面代码或篡改安装包。
- 不把终端环境变量注入作为默认方案：桌面加载环境和已有进程复用会影响实际结果。
- 未读取用户 API 密钥；未修改用户 Codex 配置、登录、历史；未重启应用；未向模型供应商发送实际请求。

## 5. 延期的实机证据

用户已明确当前只跑单元测试。本节仅保留未验证事项，不是当前执行任务；不得为补齐这些证据操作用户正在使用的 Codex App。

1. 目标 Codex App 版本在已有登录/无登录两种状态下能否选择和调用自定义 provider。
2. 固定 provider ID + 模型别名是否在新任务、旧任务恢复、多个窗口间保持准确路由。
3. 原生 Responses 和 Chat 适配分别完成文件读取、编辑、工具结果回传、多轮继续、取消、长上下文。
4. 配置/模型目录更新何时生效；可靠的公开热加载入口缺失时，用应用正常退出再启动。
5. Windows 的 Store/独立安装包识别及原生启动；macOS 多副本、名称变化、非默认路径。
6. 无法读取有效配置层或受管理员策略约束时的错误，不以覆盖策略“修复”。

以上在用户明确恢复真实测试后再安排，不作为当前单元测试阶段的前置关卡，也不宣称已经通过。
