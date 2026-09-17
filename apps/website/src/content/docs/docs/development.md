---
title: 开发、测试与文档站部署
description: 安全预览和构建 Codex Switch，运行离线测试与显式摘要验证，维护中英文文档、真实截图和网站 SEO 部署配置。
---

## 仓库结构

```text
apps/desktop/             React + Vite 桌面界面
apps/desktop/src-tauri/   Tauri 原生壳与平台能力
apps/website/             Astro + Starlight 官网和文档
crates/core/              配置事务、路由、兼容适配、SQLite
crates/cc-switch-codex/   固定版本的 CC Switch 代码、测试与许可证
packages/                共享设计变量、产品信息与服务模型预设
docs/                    设计、源码研究和历史测试报告
```

## 预览与构建

环境与原生构建依赖见[安装与下载](/docs/install/)。

```sh
pnpm install --frozen-lockfile
pnpm dev       # 桌面 UI 浏览器预览，默认端口 1420
pnpm website   # 官网与文档，默认端口 4321
```

分别在终端运行这两个开发服务。`pnpm dev` 使用页内模拟数据，不调用模型、不读写 Codex 配置或系统凭据。刷新页面会重置示例数据。`pnpm desktop` 则启动真实原生应用，会访问本机配置与凭据库，开启后修改配置。

```sh
pnpm check
pnpm test
pnpm check:upstream
cargo clippy -p codex-switch-core --all-targets --no-deps -- -D warnings
pnpm build
pnpm check:links
pnpm check:seo
pnpm check:deployment
python3 scripts/check_secrets.py --artifacts
```

离线测试使用临时目录、内存数据库、模拟凭据和合成 SSE，不启动真实 Codex。CI 不运行真实供应商测试，不需要模型密钥。`check:upstream` 校验复用源文件及提取片段，来源见[开源致谢](/docs/open-source/)。

## 最小摘要测试：只检查能否返回摘要

在根目录按 `.env.example` 创建本机 `.env`，填写要测厂商的 Key 后显式运行：

```sh
# 仅检查一个厂商的默认模型
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke deepseek
# 检查五家预设默认模型
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke --all
```

每家只发一次普通 Responses 流式请求，输入约 128 KB 合成历史，输出上限 2,048 tokens、超时 120 秒。检查 HTTP 成功、完成状态、非空摘要、流式一致性和摘要短于输入。

它不启动 Codex 客户端、不操作真实配置或会话、不调用远程 compact，也不验证摘要事实完整性或续接效果。使用的是测试预设与 `.env`，**不是自动读取桌面应用中已保存的配置**。当前结果见[普通摘要生成验证](/docs/compatibility/#普通摘要生成验证)。

## 其他真实服务测试

| 命令                                | 范围                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------- |
| `pnpm test:providers deepseek`      | 每家最多 4 次合成请求，检查文本、流、虚拟工具与结果回传；单次输出上限 256 tokens、超时 60 秒。    |
| `pnpm test:providers:summary --all` | 每家最多 3 次普通请求，检查历史召回、摘要生成及摘要续接；单次输出上限 2,048 tokens、超时 120 秒。 |
| `pnpm test:providers:compact --all` | 独立远程压缩协议检查，不能用普通摘要的通过结果替代。                                              |

这些命令会调用真实服务，可能产生 API 费用。失败后不换域名重试，不执行模型返回的工具。不要 `source .env`、在命令行传密钥或上传原始响应；提交前运行密钥扫描。普通构建和离线测试不会读取 `.env` 发请求。

## 维护文档和截图

中文文档位于 `apps/website/src/content/docs/docs/`，英文位于 `apps/website/src/content/docs/en/docs/`，使用相同文件名，内容需同步维护。导航在 `apps/website/astro.config.mjs`，截图位于 `apps/website/public/screenshots/`。更新界面后，通过浏览器预览实际操作、截图，使用虚构 Key 和明确的示例配置；不要用拼接界面或真实凭据替代。

采集方法、来源版本与各图说明见[截图记录](https://github.com/goo-yyh/codex-switch/blob/main/docs/design/screenshots.md)。截图展示界面，不承担原生功能或供应商验证结论。更新后运行构建与链接检查。

## 发布信息与文档入口

`packages/product-info/product.json` 集中保存版本、渠道、仓库、下载地址与 SHA-256。下载地址和校验值都填写后，下载页才显示按钮。正式发布前还需同步 Cargo / package / Tauri 构建版本；当前没有公开安装包地址。

| 构建变量                | 用途                                                                |
| ----------------------- | ------------------------------------------------------------------- |
| `PUBLIC_SITE_URL`       | 正式 HTTPS 域名，用于 canonical、语言版本链接、分享图片和站点地图。 |
| `PUBLIC_SITE_INDEXABLE` | 预览部署设为 `false`，页面不索引且 robots 禁止抓取。                |
| `VITE_PUBLIC_DOCS_URL`  | 应用中的独立文档 HTTPS 入口；未设置时打开源码文档。                 |

两者均为公开构建信息，不要放凭据。`pnpm build` 生成静态站点到 `apps/website/dist/`，构建成功不代表已经部署。

## 中英文网址与 SEO 部署

默认中文使用 `/` 与 `/docs/`，英文使用 `/en/` 与 `/en/docs/`。语言切换进入当前页面的对应版本，不根据浏览器语言自动跳转。首页、下载页、文档正文、导航与搜索均区分语言；截图继续使用真实中文界面，英文文档提供对应操作说明。

部署前在托管环境中设置真实域名，再构建：

```sh
PUBLIC_SITE_URL=https://your-domain.example pnpm --filter @codex-switch/website build
pnpm check:links
pnpm check:seo -- --require-site
```

上面的域名仅为示例，使用时替换为实际域名。本地未设置 `PUBLIC_SITE_URL` 时，页面标记为不索引，不生成虚构域名的 canonical 或语言版本链接。Vercel 可回退到系统提供的稳定生产域名，Preview 自动禁止索引；其他测试部署可设置 `PUBLIC_SITE_INDEXABLE=false`。

每页使用独立标题与描述，统一生成 canonical、双向 `hreflang`、默认中文 `x-default`、Open Graph、Twitter 卡片及结构化数据。构建生成 `robots.txt` 与站点地图；404 不进入站点地图。`check:seo` 检查全部页面的语言对应、元数据、结构化数据与站点地图覆盖，`--require-site` 会要求完整正式域名配置。

这些配置帮助搜索引擎理解内容，不代表页面已经被收录或保证排名。上线后需在实际域名再次验证。

Vercel 部署请阅读[部署指南](/docs/deployment/)，包含仓库根目录设置、正式域名回退、Preview 自动禁止索引和中英文 HTTP 404 路由。
