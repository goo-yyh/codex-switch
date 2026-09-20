# 厂商与模型预设

`providers.json` 是桌面原生端、浏览器预览和官网共用的连接预设。
`models.json` 是添加/编辑连接时的模型候选列表，按厂商 ID 匹配。它是人工核对官方文档后的快照，不是当前账户的授权模型列表。

## 目录版本与发布

`version.json` 保存数据版本，初始为 `1`，桌面应用与网站构建共用这一份文件。每次更新公开模型目录时，将其中的 `version` 递增（`1` → `2` → `3`），再部署网站。桌面端启动时后台检查，仅在远程版本高于本地版本时合并新增模型；相同版本（包括首次启动的 `1`）跳过合并和缓存写入，较低版本也不会回退本地目录。

`schemaVersion` 只表示 JSON 格式，`revision` 是构建自动生成的内容哈希，均不替代数据版本。只改模型数据、不递增 `version`，已安装的应用不会同步该次变更。首次上线此机制需部署带 `version` 字段的接口；旧接口缺少该字段时，桌面端继续使用本地预设。

## 模型来源与核对结果

目录核对日期：**2026-09-20**；数据版本为 **2**。此次刷新覆盖全部 5 家预设厂商，范围为通用对话、推理、编程和视觉理解模型；不包含独立的图像/视频生成、语音、向量、OCR、翻译等专用服务。每家按已核对的版本顺序最多保留 5 个候选，不足 5 个则全部保留；不展开所有日期快照、美国区域专属 ID 或平台代售的其他厂商模型。

| 厂商     | 候选数量 | 官方来源                                                                                                                                                 | 本次核对                                                    |
| -------- | -------: | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| 千问     |        5 | [文本生成模型列表](https://help.aliyun.com/zh/model-studio/model-list-text-generation/)                                                                  | 排除 8 个 Responses 失败项后，保留版本最新的 5 个          |
| MiniMax  |        5 | [Chat Completions 的 model 枚举](https://platform.minimax.cn/docs/api-reference/text-chat-openai)                                                        | 保留 M3、M2.7 和 M2.5（含高速版）                 |
| 智谱     |        5 | [模型概览](https://docs.bigmodel.cn/cn/guide/start/model-overview)、[GLM-5.3-Flash 模型编码](https://docs.bigmodel.cn/cn/guide/models/vlm/glm-5.3-flash) | 保留 5.3、5.3-Flash、5.3-FlashX、5.2、5.1                      |
| Kimi     |        4 | [模型列表及下线说明](https://platform.kimi.com/docs/models)                                                                                              | 当前公开 API 列表已完整，保留 K3、K2.7 Code/Highspeed、K2.6 |
| DeepSeek |        2 | [模型与价格、兼容别名说明](https://api-docs.deepseek.com/quick_start/pricing/)                                                                           | 仅保留当前 Flash 与 Pro，移出两个旧兼容别名            |

## 排序与兼容

- 按模型版本从新到旧排列，同版本的常规、加速、轻量、视觉等变体相邻；参数规模不是版本号。千问未标明版本的通用系列名放在末尾，不推测它们当前映射的版本。
- DeepSeek 的 V4.1-Flash 调用 ID 是 `deepseek-flash`。`deepseek-v4-flash`、`deepseek-v4-flash-vision-exp` 是指向它的兼容别名，不再占用内置候选名额；随后是 V4-Pro 的 `deepseek-v4-pro`。不能将其他平台的 `deepseek-v4.1-flash` ID 直接用于 DeepSeek 官方接口。
- 官方当前说明 `deepseek-v4-pro` 继续提供 Pro 服务；删除此前“Pro 映射到 Flash”的过时说明。
- 不重新加入 Kimi 已下线的 `kimi-k2.5`、`kimi-k2` 或 `moonshot-v1` 系列。
- 千问 Token Plan、Kimi Coding 等专用端点继续使用对应套餐的候选列表，不与普通 API 列表合并。
- 候选顺序与已选模型顺序独立。此次更新将新建 DeepSeek 连接的默认模型调整为 `deepseek-flash`，不修改已有连接的默认模型、自定义 ID 或已选顺序；已保存但不在候选中的 ID 仍可编辑。

2026-09-16 的 [Responses 实测报告](../../docs/testing/provider-responses-models-2026-09-16.md) 覆盖移除前的 81 个模型。根据用户要求，先移除千问的 8 个失败项，再按每家最多 10 个精简：千问 10、MiniMax 8、智谱 10、Kimi 4、DeepSeek 4，当日五家候选共 36 个；具体排除名单、错误类别及重新加入条件见根目录 [AGENTS.md](../../AGENTS.md)。报告和脱敏记录保留当时的完整测试范围，不随候选列表改写。

2026-09-17 按用户要求进一步缩减为每家最多 5 个：千问 5、MiniMax 5、智谱 5、Kimi 4、DeepSeek 4，共 **23 个**。套餐列表独立维护，也最多 5 个；现有套餐均未超限。保留移出候选模型的能力元数据，供已有配置和手填模型继续使用；历史核对报告不删改。

连通结果仅限当时默认地址及账户的普通非流式 Responses 文本请求，不代表已验证工具、多模态、上下文长度或压缩能力。模型能力仍由连接的默认设置及 `modelOverrides` 决定，列表调整不自动扩展能力声明。

## 2026-09-17 套餐预设补充

- 智谱 Coding Plan：`https://open.bigmodel.cn/api/coding/paas/v4`，Chat Completions，默认 `glm-5.3`，套餐候选为 `glm-5.3`、`glm-5.3-flash`。依据[官方切换指南](https://docs.bigmodel.cn/cn/coding-plan/latest-model)及[套餐概览](https://docs.bigmodel.cn/cn/coding-plan/overview)，两者均使用 1M 上下文；Flash 支持图像输入。该指南也列出 Codex 的 Responses 地址 `https://open.bigmodel.cn/api/v1`，与已有智谱预设一致。
- MiniMax Token Plan：`https://api.minimax.cn/v1`，Responses，默认及套餐候选为 `MiniMax-M3`。依据[官方 Codex 指南](https://platform.minimax.cn/docs/token-plan/codex)，能力沿用已有 M3 设置；使用[订阅 Key](https://platform.minimax.cn/docs/token-plan/quickstart)，与按量 Key 不互通。
- 新增套餐未使用真实 Key 实测；此前普通地址的 Responses 结果不能作为这些套餐的实测证据。普通模型候选、已有配置与密钥不变。

## 思考档位与默认值

2026-09-17 核对五家官方文档并补齐 36 个候选的思考配置。默认值不统一为 high；开关型模型与原生强度档位分开说明，未单独公布默认值的型号保留自动选择。完整型号、接口差异、来源和验证边界见[核对记录](../../docs/testing/model-reasoning-defaults-2026-09-17.md)。已有配置只在读取时补齐缺失字段，保留明确的用户设置。

## 上下文预留

2026-09-17 逐模型核对 36 个普通候选及 9 个套餐候选，预设采用官方容量基数的 80%，向下取整。每个候选均填写独立上下文，避免继承旗舰模型的较大值；外层默认值与默认模型一致。已有配置和用户手填值不迁移，不在运行时重复折算。

智谱 1M 的文档数值有差异，采用较小的 1,000,000 为基数；Kimi 套餐 `k3` 按 Moderato 的 262,144 权益为基数，高档会员可手动改用 838,860。完整数值、来源、别名更新及验证边界见[上下文核对记录](../../docs/testing/model-context-windows-2026-09-17.md)。

## 2026-09-20 数据版本 2

- 千问：加入 [qwen3.8-omni-flash](https://help.aliyun.com/zh/model-studio/qwen3-8-omni-flash)，移出较旧的 `qwen3.7-max`，默认仍为 `qwen3.8-max`。官方明确支持文本输出、Responses 和工具调用，因此纳入普通候选；[思考档位说明](https://help.aliyun.com/zh/model-studio/qwen-omni)列出 none / low / medium / xhigh，默认 xhigh。
- 智谱：加入 [glm-5.3-flashx](https://docs.bigmodel.cn/cn/guide/models/vlm/glm-5.3-flash)，移出 `glm-5`。FlashX 暂未进入官方 Coding Plan，套餐候选不变。
- 两个新模型官方上下文均为 1M，按现有 80% 规则预设 800000；仅声明当前客户端支持的文本和图片输入，不扩展音视频输入。新增型号及 DeepSeek 默认型号已通过[普通非流式 Responses 文本实测](../../docs/testing/provider-registry-v2-text-2026-09-20.md)；未验证工具调用、图片、SSE 或长上下文。
- DeepSeek：根据[更新日志](https://api-docs.deepseek.com/zh-cn/updates/)，普通候选仅保留 `deepseek-flash`、`deepseek-v4-pro`；新建连接默认使用 `deepseek-flash`。旧别名能力元数据保留，已有用户配置不改写。
- MiniMax、Kimi 和所有套餐列表保持不变。普通候选共 **21 个**（5 / 5 / 5 / 4 / 2）。远程更新沿用现有合并规则，保留用户参数与默认选择，不删除已有配置。
