# Codex Switch

让 Codex App 连接你选择的模型。一个具名配置对应一个 API Key 和多个模型，可同时启用多个配置。**新增配置 → 勾选配置并应用 → 打开 Codex**。

Tauri 2 + React + Rust 桌面应用，配套独立的 Astro + Starlight 官网文档。第一版为开发预览：实现与隔离测试可以运行，尚未进行真实 Codex App 端到端测试。

## 功能

- 千问、MiniMax、智谱、Kimi、DeepSeek 五家原生 Responses 预设，包含 Kimi Coding、千问 Token Plan 的独立套餐选项。
- 所有供应商地址可编辑；每个配置使用一个 API 地址，保存不会发送请求，可手动测试配置；选中模型后可点击该行的「编辑」设置能力。
- 复用 CC Switch 的 Chat 转换、SSE、工具历史和模型目录；提供远程压缩开关与显式备用队列。
- 总开关：**开启前保存原配置，关闭后精确恢复**。同次开启切换服务不覆盖原始备份；下一次开启重新备份。
- 配置外部修改冲突保护、异常退出恢复记录、原子写入。
- 系统凭据库保存 Key；SQLite 只保存连接元数据。
- 原生打开 / 正常重启 Codex、托盘快速开关与切换配置、后台运行、可选登录启动。
- 原创简明界面、官网首页、下载说明、12 篇可搜索产品文档。

## 安装依赖

需要 Node.js 22.12+、pnpm 10、Rust stable。原生构建还需要 Xcode Command Line Tools（macOS）或 Microsoft C++ Build Tools / WebView2（Windows）。

```sh
pnpm install --frozen-lockfile
```

## 安全预览

```sh
pnpm dev       # http://127.0.0.1:1420，桌面界面内存模拟
pnpm website   # http://127.0.0.1:4321，官网与文档
```

浏览器中的桌面预览不读取密钥、不联网调用模型、不修改 Codex 配置。只有启动原生应用后才会接入本机功能。

```sh
pnpm desktop   # 真实原生应用，会访问本机配置与凭据库
```

## 验证与构建

```sh
pnpm check
pnpm test
pnpm check:upstream
cargo clippy -p codex-switch-core --all-targets --no-deps -- -D warnings
pnpm build
pnpm check:secrets
pnpm --filter @codex-switch/desktop tauri build
```

`pnpm build` 构建网页界面和官网；Tauri 命令生成原生包，输出到 `target/release/bundle/`。Windows 使用：

```sh
pnpm --filter @codex-switch/desktop tauri build --config src-tauri/tauri.windows.conf.json
```

当前没有公开签名安装包。macOS 本地构建未经 Apple 公证；Windows 原生实机验证和正式分发签名留待发布阶段。CI 只构建与上传测试产物，不自动发布版本。

## 真实供应商测试

根目录 `.env` 使用 `.env.example` 中的空变量模板，实际值留在本机。

```sh
pnpm test:providers --all
pnpm test:providers deepseek
pnpm test:providers:summary --all # 显式调用真实服务：普通摘要压缩与续接
```

每家最多 4 次合成请求，单次输出上限 256 tokens、60 秒超时；依次检查文本、流、虚拟函数调用和结果回传。失败后停止该服务，不切换域名重试，不执行模型返回的工具。请求可能产生少量费用。

`test:providers:summary` 是独立的长上下文检查：每家最多 3 次普通 Responses 请求，约 128 KB 合成历史、单次输出上限 2,048 tokens、120 秒超时。验证原始历史召回、文本摘要生成、仅凭摘要续接及 token 减少；不调用 remote compact，也不启动 Codex App。报告见 [普通摘要压缩验证](docs/testing/provider-summary-validation-2026-09-16.md)。

设置 → 通用设置 → **远程上下文压缩** 默认关闭，Codex 通过普通模型请求执行摘要压缩。用户开启后，配置采用 CC Switch 的远程压缩标记，由 Codex 客户端选择协议；本项目透传原生 Responses 的 Remote V2 `compaction_trigger` 和旧版 compact 请求，不保证上游支持或失败时自动回退。开关保存后下次开启 Codex Switch 时生效。

配置页的 API 地址和接口格式使用厂商预设作为初始值，可并排修改；勾选模型不会改变外层配置。打开 API 地址标签右侧的 **完整 URL** 后，普通请求直接使用填写的地址，不追加 `/responses` 或 `/chat/completions`。模型编辑弹窗可分别覆盖地址、接口格式和完整 URL 模式，也可恢复继承外层设置；上下文长度只在模型弹窗中编辑。手动测试和实际路由均使用各模型的有效设置。更换已保存配置的请求地址（包括单个模型的地址）需要重新填写密钥；保存不会自动发送测试请求。

密钥仅由显式测试入口读取；普通测试和 CI 不读取 `.env`。不要使用 `source .env`、不要上传凭据或原始响应。提交前可运行 `python3 scripts/check_secrets.py --artifacts` 检查源码和构建产物。

## 测试边界

当前工作不启动、重启或终止真实 Codex App，不读写真实 Codex 配置、认证、会话和系统凭据。配置测试只使用临时目录；路由测试使用内存请求和本地合成 HTTP 服务。供应商 HTTP 检查是单独的显式命令。

连通测试通过只证明当次接口检查通过，不能推导完整 Codex App 兼容。图片能力按模型目录声明，Chat 的工具搜索与图片转换复用上游。原生 remote 请求透传；旧版 Chat compact 路由仍走上游普通转换路径，不代表已支持远程契约。Chat 的 V2 触发项或加密压缩历史明确报错，不静默丢弃，也不伪造压缩项。普通摘要请求可走正常 Chat 桥接。备用队列默认关闭，不自动猜测接口或域名；转换缓存有界且不跨进程保存。

## 目录

| 目录                     | 内容                                      |
| ------------------------ | ----------------------------------------- |
| `apps/desktop`           | React 界面与 Tauri 原生壳                 |
| `crates/core`            | 配置恢复事务、网关、兼容适配、存储        |
| `crates/cc-switch-codex` | 固定版本的 CC Switch 模块、原测试与许可证 |
| `apps/website`           | 官网与独立产品文档                        |
| `packages`               | 共享设计变量、服务预设                    |
| `docs`                   | 技术方案、源码研究、测试记录              |

- [技术方案](docs/product-plan.md)
- [界面规范](docs/design/ui-spec.md)
- [CC Switch 源码研究](docs/research/cc-switch.md)
- [测试记录](docs/testing/implementation-validation.md)
- [单元测试方案](docs/testing/unit-test-plan.md)
- [供应商测试方案](docs/testing/provider-test-plan.md)

## 实现与配置恢复

只投影 `config.toml`，不修改 `auth.json`。原始内容保存在应用数据目录的恢复记录中。关闭后原文件按字节恢复，原先不存在则删除本次创建的文件。配置冲突必须先保留当前版本才能恢复。

关闭窗口会继续在托盘提供连接；完全退出前需先正常退出 Codex，并恢复原配置。异常退出后重新打开应用，可恢复旧配置或重新建立路由。

本项目面向 Codex App，但 Codex 客户端可能共用配置目录；不为 CLI / IDE 提供适配保证。官网不依赖 codex-docs.com，发布时可通过 `PUBLIC_SITE_URL` 设置自己的站点域名。

## 许可

MIT。兼容内核直接复用 [CC Switch](https://github.com/farion1231/cc-switch) 固定提交的源码与测试，保留上游 MIT 版权声明。来源及本地差异见 [UPSTREAM.md](crates/cc-switch-codex/UPSTREAM.md)。Codex Switch 是社区独立项目，与 OpenAI 无隶属关系。
