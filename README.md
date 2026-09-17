# Codex Switch

让 Codex App 连接你选择的模型。**一个配置、一个 API Key、多个模型**，首页可同时启用多个配置；开启前备份，关闭后恢复原配置。

支持 macOS / Windows，使用 Tauri 2、React 和 Rust 构建，配套 Astro + Starlight 文档站。当前为 **0.1.0 开发预览版**，尚未提供公开签名安装包，也未完成真实 Codex App 端到端验证。

[快速开始](apps/website/src/content/docs/docs/quickstart.md) · [配置说明](apps/website/src/content/docs/docs/configuration.md) · [安装与构建](apps/website/src/content/docs/docs/install.md) · [兼容范围](apps/website/src/content/docs/docs/compatibility.md)

![Codex Switch 实际界面：多选配置并开启连接](apps/website/public/screenshots/enabled.png)

_截图来自当前应用的浏览器预览，使用示例配置；原生连接、备份和进程状态为模拟数据。[截图记录](docs/design/screenshots.md)_

## 能做什么

- **管理多个服务**：内置千问、MiniMax、智谱 GLM、Kimi、DeepSeek，提供 Kimi Coding 与千问 Token Plan 套餐选项，也可填写自定义 HTTPS 接口。
- **按模型配置**：一个配置可选 1–20 个模型，指定默认模型，逐模型覆盖地址、接口格式、上下文长度、思考档位、图像与并行工具能力。
- **兼容两种接口**：支持原生 Responses 转发，以及 Chat Completions 的本地协议转换。请求失败后不会自动切换服务或协议。
- **可恢复的配置切换**：开启前保存原始 `config.toml`，关闭后按字节恢复；检测外部修改冲突，不改写 `auth.json` 或会话文件。
- **在本机运行**：Key 存入系统凭据库，请求由本机直接发往选定服务；支持托盘、后台运行、可选登录启动与正常打开 / 重启 Codex。

## 三步连接

1. **新增配置**：选择服务及套餐，填写配置名称、API 地址和对应的 Key。
2. **选择模型并保存**：勾选模型，按需编辑能力。保存不会发请求；「测试配置」是独立、手动触发的连通检查。
3. **勾选配置并开启**：首页至少选中一个配置，打开总开关，再点击「打开 Codex」。模型以 `配置名称-模型` 显示。

新增、编辑和删除前需关闭总开关。开启期间可切换已保存的配置；关闭窗口仍在后台提供连接。完整步骤见[截图教程](apps/website/src/content/docs/docs/quickstart.md)，恢复行为见[开启、关闭与恢复](apps/website/src/content/docs/docs/switch.md)。

## 配置要点

| 设置           | 怎么填 / 有什么作用                                                                                      |
| -------------- | -------------------------------------------------------------------------------------------------------- |
| API 地址       | 默认填写基础地址，应用追加 `/responses` 或 `/chat/completions`，保留已有路径前缀。                       |
| 完整 URL       | 开启后普通请求直接使用填写的 URL，不追加路径；用于服务商给出的完整请求端点。                             |
| 接口格式       | 根据目标服务选择 Responses 或 Chat Completions，不会失败后自动互换。                                     |
| API Key        | 同一配置的模型共用一个 Key。编辑时留空可保留原 Key；更换请求地址或完整 URL 模式需重新填写。              |
| 模型设置       | 默认继承外层地址与接口，可在模型行的「编辑」中覆盖；确认后还需保存整个配置。                             |
| 远程上下文压缩 | 默认关闭，建议保持关闭；关闭时仍可通过普通模型请求生成摘要。开关自动保存，下次开启 Codex Switch 时生效。 |

详细说明：[配置字段与模型能力](apps/website/src/content/docs/docs/configuration.md) · [套餐与自定义接口](apps/website/src/content/docs/docs/relay.md) · [通用设置](apps/website/src/content/docs/docs/settings.md)。能力声明不等于服务实测通过，远程压缩尤其需要上游支持。

## 开源致谢：CC Switch

本项目参考并直接复用了 [CC Switch](https://github.com/farion1231/cc-switch) 的部分开源代码和测试，感谢作者及贡献者。复用内容包括 **Chat / Responses 转换、SSE 处理、工具历史与 schema 辅助逻辑、模型目录及能力模板**。

- 固定上游提交：[`06082e189d65e6d6dbadc35dacdac1ce6c79d89a`](https://github.com/farion1231/cc-switch/tree/06082e189d65e6d6dbadc35dacdac1ce6c79d89a)。
- 源码位于 [`crates/cc-switch-codex`](crates/cc-switch-codex)，保留[上游 MIT 许可证](crates/cc-switch-codex/LICENSE)及版权声明。
- 复用范围、本地差异和更新方式见 [UPSTREAM.md](crates/cc-switch-codex/UPSTREAM.md)；文件与片段通过清单校验。

Codex Switch 的界面、配置备份与恢复、凭据管理、HTTP 路由生命周期和文档站由本项目维护。该模块是源码提取，不是 CC Switch 官方 SDK；本项目是社区独立项目，与 OpenAI 无隶属关系。

## 本地开发

需要 Node.js 22.12+、pnpm 10、Rust stable。原生构建另需 macOS Xcode Command Line Tools，或 Windows Microsoft C++ Build Tools / WebView2。

```sh
git clone https://github.com/goo-yyh/codex-switch.git
cd codex-switch
pnpm install --frozen-lockfile
pnpm dev       # 桌面界面浏览器预览，默认 http://127.0.0.1:1420
pnpm website   # 官网与文档，默认 http://127.0.0.1:4321
```

以上两个开发服务分别在终端运行。浏览器预览仅使用页内模拟数据，不读取密钥、不调用模型、不修改 Codex 配置。`pnpm desktop` 启动真实原生应用，会访问本机配置与系统凭据库。

```sh
pnpm check
pnpm test
pnpm check:upstream
cargo clippy -p codex-switch-core --all-targets --no-deps -- -D warnings
pnpm build
pnpm check:links
pnpm check:secrets
```

`pnpm build` 构建界面和文档站。原生包输出到 `target/release/bundle/`：

```sh
# macOS
pnpm --filter @codex-switch/desktop tauri build
# Windows
pnpm --filter @codex-switch/desktop tauri build --config src-tauri/tauri.windows.conf.json
```

文档站可通过 `PUBLIC_SITE_URL` 设置部署地址；应用通过 `VITE_PUBLIC_DOCS_URL` 设置文档入口。详见[开发与测试](apps/website/src/content/docs/docs/development.md)。

## 验证与限制

离线测试使用临时配置目录、模拟凭据与本地合成 HTTP 服务，不操作真实 Codex。真实供应商测试需按 `.env.example` 配置本机 `.env` 后显式运行，可能产生 API 费用：

```sh
pnpm test:providers deepseek # 文本、流式、虚拟工具与结果回传
# 最小摘要验证：每家默认模型仅发送一次普通摘要请求，不启动 Codex
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke --all
```

2026-09-17 的[最小摘要验证](docs/testing/provider-summary-smoke-2026-09-17.md)中，五家预设默认模型均返回完成状态和非空摘要。它只证明当次摘要生成成功，不代表摘要事实完整性、续接效果、远程压缩或 Codex App 端到端兼容。更多测试命令和范围见[开发文档](apps/website/src/content/docs/docs/development.md)。

公开签名、公证与 Windows 实机验证仍待完成；CI 构建产物不等于正式发布。Codex 工具可能共用配置目录，本项目暂不为 CLI / IDE 扩展提供适配保证。

## 目录与许可

| 目录                     | 内容                                    |
| ------------------------ | --------------------------------------- |
| `apps/desktop`           | React 界面与 Tauri 原生壳               |
| `crates/core`            | 配置恢复事务、网关、兼容适配、存储      |
| `crates/cc-switch-codex` | 固定版本的 CC Switch 模块、测试与许可证 |
| `apps/website`           | 官网、使用文档和界面截图                |
| `packages`               | 共享设计变量、服务与模型预设            |
| `docs`                   | 技术方案、源码研究、测试记录            |

[技术方案](docs/product-plan.md) · [供应商测试方案](docs/testing/provider-test-plan.md) · [开源致谢](apps/website/src/content/docs/docs/open-source.md)

本项目使用 [MIT 许可证](LICENSE)，第三方代码保留各自版权与许可声明。
