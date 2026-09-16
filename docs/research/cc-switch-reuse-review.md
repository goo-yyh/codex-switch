# CC Switch 复用复核

日期：2026-09-16。范围：Codex 配置与模型目录、供应商预设、协议转换、流式事件、历史续接、压缩路由、故障转移及模块依赖。

> 以下是实施前的研究快照；其中“当前”“没有”“未运行”描述研究当时的状态。后续开发已完成模块接入，见 [开发与验证记录](../testing/cc-switch-integration.md)。

## 结论

Codex Switch 应复用 CC Switch 的 Codex 兼容实现及配套测试，保留自己的界面和产品流程。独立产品并不要求独立重写协议内核。当前以简化 Chat 转换器覆盖全部国内供应商的做法，没有充分利用上游已经实现的原生 Responses 路径和供应商适配。

此前“参考架构、独立编写代码”的实现选择导致兼容逻辑重复建设。已有审查中的推理字段恢复、长命名空间工具名、显式并行参数等问题，上游都有相关处理。这些是已记录的重复实现成本，不是需要继续重写的理由。参见 [此前实现审查](../reviews/2026-09-15-implementation-review.md)。

本次完成源码与测试定义研读、依赖边界分析及复用清单；未移植运行代码，未运行上游测试，也未验证真实 Codex App。本项目已有地址编辑相关测试的执行结果另见末节。

## 证据基线

- 仓库：`farion1231/cc-switch`。
- 本地参考：`/tmp/codex-switch-cc-switch-reference`，工作树干净。
- 固定提交：`06082e189d65e6d6dbadc35dacdac1ce6c79d89a`；不是对最新主分支的声明。
- 源码许可证为 MIT，版权为 `Copyright (c) 2025 Jason Young`。移植时保留原许可证和版权，记录来源提交与本地修改。
- 本项目仍没有 Git 提交历史，因此对原实现取舍的解释来自当前代码与项目记录，而非不可变的历史提交。

## 1. CC Switch 是配套方案，不只是接口转发

### 供应商、套餐和协议一起配置

[`codexProviderPresets.ts`](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src/config/codexProviderPresets.ts) 将端点、模型、`apiFormat`、推理档位、上下文、并行工具和输入模态放在同一预设中。Kimi 普通 API 和 Coding 套餐分别配置；DeepSeek、GLM、MiniMax 有原生 Responses 预设。

例如 GLM 原生 Responses 预设使用 `https://open.bigmodel.cn/api/v1`，而本项目当前默认是 Chat 地址 `https://open.bigmodel.cn/api/paas/v4`。协议选择必须与套餐、地址一起处理，不能只改一个枚举。

### 模型目录决定 Codex 发出哪些工具

[`CodexCatalogToolProfile`](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/codex_config.rs#L394) 区分 `ProxyChat`、`NativeResponses`、`Anthropic`。原生路径与转换路径使用不同模型模板；原生路径通常去掉不支持的 freeform/custom 工具并采用 shell 编辑。DeepSeek 另有官方模型目录镜像路径，保留该供应商的工具与指令组合，而非一律剥离能力。

本项目 `providers::catalog()` 统一设置无推理档位、文本输入、不支持并行工具的模板，没有迁入上述模型级策略。

### 只有需要转换的连接进入转换层

[`codex.rs`](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/providers/codex.rs#L25) 根据 provider 元数据、配置和地址判断上游协议。Codex 客户端侧继续使用 Responses；原生 Responses 可直连或经代理透传，Chat/Anthropic 才进行相应转换。不能把“Codex 只发 Responses”理解成所有上游都需要 Chat 桥接。

## 2. 应连同测试复用的兼容内核

| 模块 | 已实现的行为 | 本次统计的测试属性数 |
| --- | --- | ---: |
| `transform_codex_chat.rs` | 请求/响应、namespace、custom、tool search、工具结果、多种 reasoning 字段、参数与错误映射 | 97 |
| `streaming_codex_chat.rs` | SSE 状态、文本/推理/工具增量、工具名还原、截断状态、缺失 index 的分片归并 | 26 |
| `codex_chat_history.rs` | previous response 和唯一 call ID 补全、原推理字段恢复、歧义 ID 拒绝恢复 | 8 |
| `codex_responses_sse.rs` | Responses 事件构造 | 5 |
| `json_canonical.rs` | JSON 参数规范化与稳定哈希 | 7 |
| `sse.rs` | SSE 边界与 UTF-8 处理 | 14 |
| `transform_codex_chat_moonshot_schema.rs` | Moonshot/Kimi JSON Schema 的 `$ref` 同级字段兼容 | 9 |

前三个模块共 131 个测试属性。这里统计的是源码中的 `#[test]` / `#[tokio::test]`，不代表本轮执行通过，也不以数量代替质量判断。

抽查的用例包括 `responses_request_to_chat_exposes_tool_search_and_loaded_namespace_tools`、`chat_response_truncated_stays_incomplete_instead_of_error`、`missing_index_argument_fragments_stay_in_one_call`、`does_not_restore_ambiguous_call_id_without_previous_response`。

Kimi Schema 模块尤其与桌面产品相关：注释记录 Codex Desktop 的内置工具会产生 `$ref` 与其他关键字同级的 Schema，目标端点拒绝该格式；上游把 `$ref` 移入 `allOf`，保留其他字段，仅对匹配的 Chat 端点应用。源码引用了问题 #6867。本次确认了实现和测试存在，没有重新进行供应商实测。

## 3. 备用机制的真实含义

[`provider_router.rs`](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/provider_router.rs#L45) 根据 `auto_failover_enabled` 决定只使用当前 provider，还是按用户配置的故障转移队列选择供应商。官方账号连接有单路由保护。

[`forwarder.rs`](https://github.com/farion1231/cc-switch/blob/06082e189d65e6d6dbadc35dacdac1ce6c79d89a/src-tauri/src/proxy/forwarder.rs#L428) 配合重试上限、熔断许可和错误分类处理可重试失败；不可重试错误和客户端中断不会继续轮换供应商。流式响应至少在初始读取成功后才交给下游，后续流中断不能概括为能够无缝重放。该文件还包含其他客户端协议的专用验证，不能把它们全部算作 Codex 路径已具备的行为。

这不是“原生 Responses 失败后自动猜测 Chat 地址”。协议来自每个 provider 的配置，备用 provider 可以有自己的端点和协议。若采用该功能，应复用队列、错误分类和测试，而不是新增一个遇错即换协议的分支。

## 4. 压缩结论的边界

CC Switch 不仅有后端 compact 路由，还有显式的前端远程压缩开关：`CodexConfigSections.tsx` 调用 `providerConfigUtils.ts` 中的 `setCodexRemoteCompaction()`。开启时把当前自定义 provider 表的 `name` 写为 `OpenAI`，关闭时恢复供应商显示名；不改写保留的内置 provider。其配置辅助函数有相应测试，中文提示明确为“使 Codex 尝试使用远程压缩”，没有承诺任意上游都支持。普通供应商预设的 name 通常是厂商名，不能从注册了 compact 路由推断所有连接默认都走远程压缩。这是此前只阅读后端路由时遗漏的重要部分。

`handle_responses_compact_for_app()` 复用转发器。原生路径转发 `/responses/compact`；Chat 路径把该地址改写为 `/chat/completions`，使用普通请求/响应转换器。路径改写有单元测试。

此前从“普通转换器未生成加密 compaction 项”进一步评价整个 CC Switch 压缩方案，是证据不足的外推。实际效果还取决于 Codex 版本、provider 身份、采用的压缩路径、请求内容和客户端接受的返回形式。本次只确认这些代码路径，不宣称这条 Chat compact 路径已通过或必然失败。

本项目目前 Chat compact 直接 501，是与上游不同的行为。迁移时应先保留上游行为，补充目标 Codex 请求/返回契约验证；确认缺陷后再形成独立补丁，不应继续凭接口名称自行设计替代算法。

本轮之前的供应商探测只覆盖本项目五个默认模型和当前预设地址。MiniMax 返回压缩项，但摘要回忆验证未通过；其他地址返回 404/405。这不能推翻 CC Switch 使用其他套餐地址、模型目录及客户端路径的方案，也不能作为全部型号均不支持的证据。

## 5. 如何复用而不再次重写

CC Switch 当前是完整 Tauri 应用，不是独立协议 SDK：`lib.rs` 中的 `proxy`、`provider`、`codex_config` 为私有模块；Cargo 依赖包含 Tauri、数据库、网络、日志等整套运行环境。直接依赖其整个应用库不是一个现成、稳定的导入接口。

建议以固定上游提交提取 Codex 兼容模块，保留原文件结构、核心逻辑和测试，在外层接入本项目的数据结构、凭据和 HTTP 服务。可使用专门的 vendored 模块目录或 crate；具体目录在实施时确定。提取模块和最小依赖属于代码复用，不是按行为重新写一份。

| 部分 | 处理方式 |
| --- | --- |
| 供应商预设、套餐地址、协议、模型目录与 reasoning 策略 | 迁入上游结构及数据；按产品范围选取，保留来源版本 |
| Chat 转换、SSE、历史续接、Schema 适配及辅助函数 | 原文件连同测试移植，尽量只改 import、类型适配与依赖边界 |
| Native Responses 路由、目录选择和必要修正 | 沿用上游策略，接入当前应用控制流程 |
| 故障转移 | 若纳入产品，复用队列/重试/熔断实现与测试，不自造隐式协议降级 |
| UI、配置列表、系统凭据库、现有配置备份恢复 | 保留本项目产品实现，通过适配层对接 |
| 本项目缓存隔离、大小/过期限制、事务补偿等已有约束 | 明确记录为本地差异，继续保留相应测试 |
| 上游多客户端管理、推广排序、使用量面板等 | 不属于当前 Codex 专用产品所需依赖 |

建立 `UPSTREAM.md` 或等价来源清单，记录提交、文件、依赖、测试、本地修改及更新方式。移植时同时加入许可证与版权说明，并更新 README 中“代码独立编写”的陈述。后续更新以比较上游变更和小型补丁为主，避免把提取代码再次改造成难以合并的第二套实现。

## 6. 实施顺序与验收

1. 固定来源及模块依赖；先把现有测试作为行为基线保留。
2. 提取纯转换模块及上游测试，在隔离环境中编译执行；解决依赖适配，不改写算法以使测试迎合当前实现。
3. 迁入预设/套餐/模型目录配套数据，再替换请求、响应、流式和历史续接入口；迁移必须保证每个入口只有一个明确实现。
4. 验证普通调用、工具定义与回传、推理字段、流式截断、缓存冲突、压缩请求契约、错误分类和配置失败恢复。
5. 分开报告“上游测试移植通过”“本项目集成通过”“供应商接口通过”“Codex App 端到端通过”。固定版本兼容结果不自动等于所有模型和未来版本兼容。

本轮无需修改真实 Codex 配置、启动或终止 App，也不需要继续发送供应商请求。研究结果足以确定复用方向；尚未执行的移植与运行验证不能标记为完成。

## 7. 地址可编辑与预设边界

用户补充要求：供应商地址必须能够修改，不能完全受内置预设限制。

### CC Switch 的具体实现

- `CodexFormFields.tsx:780`：普通第三方连接直接展示可编辑 `EndpointField`，同时提供完整 URL 开关、管理和测速入口。托管 OAuth 连接存在端点由 adapter 决定的例外，不能把其策略套给普通 API Key 连接。
- `hooks/useCodexConfigState.ts:249`：地址变更写回当前 TOML 的 `base_url`；编辑初始化从已保存配置提取地址，不重新取预设覆盖。
- `ProviderForm.tsx:1992`：用户显式选择预设时，填入该预设配置、模型目录和协议；后续编辑属于用户配置。
- `hooks/useSpeedTestEndpoints.ts`：候选地址集合包括当前地址、编辑前保存的地址和预设的 `endpointCandidates`；用户保存的自定义地址由地址管理组件独立加载。候选地址管理与供应商故障转移队列是两种机制。
- `ProviderForm.tsx:1826` 与 `forwarder.rs:1487`：完整 URL 模式有单独元数据和 URL 组装分支。正常基础地址按协议拼接路径；完整 URL 不使用普通盲拼规则。特殊独立端点仍有自己的路径转换/拒绝逻辑，不能简单删除所有地址规范化。
- 上游还保留 TOML 编辑入口及模型能力字段的编辑保存回环；地址、协议、模型目录各有自己的状态。

### 当前 Codex Switch 已有和欠缺的部分

| 能力 | 当前事实 |
| --- | --- |
| 内置供应商自定义地址 | 已有，在 `App.tsx:545` 的高级设置中 |
| 自定义中转站地址 | 已有，在基础表单中 |
| 用户地址进入实际请求 | `save()` 提交 form，`prepare_profile()` 校验并保留 endpoint，`Profile::routes()` 把地址传给 Connection，网关从 Connection 组装 URL |
| 修改接口格式 | 已有 Responses / Chat 选择，变更协议本身不重置地址 |
| 用户显式换预设 | 当前会重置地址、协议、模型与上下文；仅点选同一预设不会重置 |
| 完整 URL 模式 | 没有独立模式；目前会去掉已知端点后缀，再拼接接口路径 |
| 多候选地址管理、测速 | 没有 |
| 地址限制 | 只允许 HTTPS，不允许查询参数、片段、URL 内嵌账号；拒绝本地回环地址。因而不等价于 CC Switch 的地址兼容范围 |
| 地址与密钥 | 地址或供应商范围变更需要重新填写密钥，是当前凭据隔离策略；不是把地址锁死 |

### 复用时应形成的产品规则

1. 普通 API Key 供应商统一提供可编辑 API 地址；预设只提供创建初值和显式选用的候选值。
2. 已保存的用户地址是运行时来源；修改模型、编辑保存、切换连接和更新预设不得隐式恢复官方地址。用户明确重新应用预设另行处理。
3. 地址、协议、套餐、模型和凭据属于一份用户配置。不能因为仍选择某个厂商图标，就无条件套用官方端点的所有能力和参数修正。
4. 基础地址、完整 URL、候选地址管理应复用上游解析/组装规则及测试，不继续扩展临时字符串拼接。
5. 候选地址测速与故障转移分别表达；连接失败不静默发送到另一个未配置的地址。
6. 针对用户覆盖值补充创建→编辑→保存→应用的回归验证，特别是自定义前缀、尾斜杠、完整端点、套餐路径和配置更新后的保留行为。

这些属于接下来实施复用时的明确约束，不表示本轮已经加入完整 URL 或多地址功能。

### 本轮验证

执行 `pnpm --filter @codex-switch/desktop exec vitest run src/App.test.tsx -t 'requires a fresh key|supports a named relay'`：2 个测试通过，22 个未选中。覆盖地址变更后的密钥处理和自定义中转站多模型保存流程；测试使用模拟 bridge，不证明真实数据库、网络地址或 Codex App 已验证。完整的地址保存回环与上游测试迁移仍需在实现阶段完成。
