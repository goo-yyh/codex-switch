# CC Switch 接入开发记录

日期：2026-09-16。来源固定为 `06082e189d65e6d6dbadc35dacdac1ce6c79d89a`。

## 已实现

- 独立 `cc-switch-codex` crate：直接复用上游 Chat 请求/响应、SSE、历史恢复、Kimi Schema、模型能力、目录模板与构建函数、熔断器，以及相应原测试。运行时已切换到新库，原独立 Chat 转换状态机已移除。
- 新建配置采用配套的原生 Responses 地址、默认模型及能力；Kimi Coding、千问 Token Plan 单独选用。已有配置保持原地址、协议、模型、上下文与凭据。
- 所有供应商 API 地址可编辑，每个配置只显示一个 API 地址输入框，默认按基础地址使用，底部按钮可手动测试所有已选模型，保存不发请求。协议改变不会自行重置地址；地址或完整 URL 范围改变需重新提供密钥。
- 逐模型上下文、思考档位及默认档位、图像和并行工具能力。Chat 专用思考参数可显式选择，不再按供应商图标强制关闭思考。
- 全局远程压缩开关沿用 CC Switch provider 标记。原生 compact 透传，Chat compact 按上游走 Chat 转换；不伪造加密压缩项。
- 显式备用队列与上游熔断器，每项有独立协议、模型、地址及凭据；响应开始前按限定错误尝试下一项，流开始后不重放。默认关闭，不隐式降级。
- 新字段 serde 默认值兼容旧 SQLite JSON；旧配置不自动继承新预设。保留配置写入补偿、恢复、缓存过期/隔离等约束。
- 源码、片段、预设来源校验与原 MIT 许可证；README/产品文档同步更新。

实现差异和升级方式见 `crates/cc-switch-codex/UPSTREAM.md`。上游完整 ProviderRouter/数据库/多客户端 forwarder 不属于这个提取库；队列和 HTTP 错误边界在本项目适配层实现，不能声称全部上游应用被直接嵌入。

## 验证边界

Rust 测试使用临时目录、临时 SQLite、内存请求和 loopback 合成服务；前端使用模拟 bridge。隔离集成覆盖 compact 不透明内容透传、URL 前缀/查询、队列协议和密钥隔离、禁止隐式降级、流中断、截断工具、模型目录、远程开关与恢复。上游模块自己的测试与本地适配层测试分别执行。

浏览器只运行 Vite 内存预览，不读取 vault、不调用供应商、不修改真实 Codex 配置。没有进行新预设的真实供应商或 Codex App 长任务端到端验证；历史 Chat 型号测试不能转移到新原生型号。

验收命令：

```sh
pnpm test
pnpm check
pnpm check:upstream
cargo clippy -p codex-switch-core --all-targets --no-deps -- -D warnings
cargo check -p codex-switch-desktop
pnpm build
python3 scripts/check_secrets.py --artifacts
pnpm check:links
```

## 本轮结果

- 前端 39 项、上游模块 190 项、本地核心 77 项、合成 HTTP/SQLite 集成 14 项、Tauri 托盘逻辑 2 项，共 322 项测试通过。
- `pnpm check`、桌面/官网 `pnpm build`、原生 `cargo check`、本地 Rust 格式与 Clippy、上游来源校验、构建产物密钥扫描、15 个构建页面的链接检查通过。
- Chromium 内存预览实际操作：Kimi → Kimi Coding 套餐；改写自定义完整 URL 与查询参数；保存再编辑保留该地址、完整 URL 开关与密钥留空语义；保存远程压缩开关和有序队列。检查 800×740 桌面尺寸。截图保存在本机忽略目录 `output/playwright/`。
- 浏览器仅出现已有 `/favicon.ico` 404；没有新增运行时异常。官网构建有缺省 `site` 跳过 sitemap 和工具链环境告警，不影响构建、类型和链接检查。
- 没有构建或发布签名安装包，没有提交/推送代码，没有实际供应商或 Codex App 端到端结论。
