---
title: 将中英文文档部署到 Vercel
description: 从仓库根目录部署 Codex Switch 静态官网与中英文文档，配置正式域名、Preview SEO 和按语言返回的 HTTP 404 页面。
---

仓库已经提供根目录 `vercel.json`。Vercel 只安装前端依赖并构建官网，不构建桌面应用或 Rust，也不需要任何模型 API Key。

## 导入项目

1. 在 Vercel 导入 `goo-yyh/codex-switch` 仓库。
2. **Root Directory 保持仓库根目录 `.`**，不要选择 `apps/website`，因为构建使用工作区共享包与根目录锁文件。
3. Framework Preset 使用 **Other**，其余构建设置由 `vercel.json` 管理。
4. 在项目环境变量中启用 **Automatically expose System Environment Variables**（系统环境变量），再部署。

| 设置             | 仓库中的值                                         |
| ---------------- | -------------------------------------------------- |
| Node.js          | `22.x`，由根目录 `package.json` 指定               |
| Package manager  | `pnpm@10.26.0`，由 `packageManager` 指定           |
| Install Command  | `npx --yes pnpm@10.26.0 install --frozen-lockfile` |
| Build Command    | `npx --yes pnpm@10.26.0 build:website`             |
| Output Directory | `apps/website/dist`                                |

网站是静态 Astro + Starlight 产物，无需添加 Serverless Function 或 Vercel SSR adapter。不要配置把所有路径重写到首页的 SPA 规则，否则不存在的页面可能被当作成功页面返回。

## 正式域名和预览部署

推荐在 Vercel 的 **Production** 和 **Preview** 环境中都设置 `PUBLIC_SITE_URL=https://你的正式域名`。绑定自定义域名后，在下一次构建中生效。

| 变量                               | 行为                                                                    |
| ---------------------------------- | ----------------------------------------------------------------------- |
| `PUBLIC_SITE_URL`                  | 优先使用的正式 HTTPS 站点根地址，不带子路径、查询参数或凭据。           |
| `VERCEL_PROJECT_PRODUCTION_URL`    | 未显式配置域名时，使用 Vercel 提供的稳定生产域名，自动补上 `https://`。 |
| `VERCEL_ENV` / `VERCEL_TARGET_ENV` | 自动区分 Production、Preview 和自定义环境；仅正式环境允许索引。         |
| `PUBLIC_SITE_INDEXABLE=false`      | 即使正式部署也可手动禁止索引。不能用 `true` 解除 Preview 的禁止索引。   |

不会将每次变化的 `VERCEL_URL` 用作 canonical。Preview 即使读取到正式域名，页面仍标记 `noindex, nofollow`，`robots.txt` 也禁止抓取；canonical 和语言链接使用稳定的正式域名。

Vercel Production 缺少显式或系统提供的正式域名时，构建会报错并说明配置方法。本地没有域名时仍可正常构建预览，默认不索引。修改域名或 SEO 变量后需要重新构建部署。

## 中英文 404

先匹配已有页面与静态资源，再匹配错误页：

| 请求示例                             | 返回                                 |
| ------------------------------------ | ------------------------------------ |
| `/docs/not-found/`、`/missing`       | 中文 404，HTTP 404                   |
| `/en/docs/not-found/`、`/en/missing` | 英文 404，HTTP 404                   |
| `/english/missing`、`/fr/missing`    | 默认中文 404，不把相似前缀误判为英文 |
| `/404.html`、`/en/404/`              | 对应语言的错误页，HTTP 404           |

错误页保留首页、文档入口和语言切换，不跳转回首页；浏览器中的原请求地址保持不变。404 页面不提供 canonical、hreflang 或结构化数据，也不进入站点地图和站内搜索。

## 本地检查

```sh
pnpm build:website
pnpm check:links
pnpm check:seo
pnpm check:deployment
pnpm preview:website
```

最后一个命令在 `http://127.0.0.1:4324` 启动静态路由预览，可打开 `/en/docs/not-found/` 和 `/docs/not-found/` 检查两种 404。普通 `pnpm website` 开发服务也保留请求语言；静态部署以 `vercel.json` 为准。

`check:deployment` 使用构建产物与实际 `vercel.json`，通过本地 HTTP 检查正常页面、静态资源、双语 404、相似前缀、GET / HEAD 和索引策略。它不是 Vercel 云端运行时的验证。

## 上线后验收

在实际部署域名检查：首页和英文首页可访问，正常页面为 HTTP 200；中英文不存在的路径均为 HTTP 404 且语言正确。确认 `/robots.txt`、`/sitemap-index.xml`、页面 canonical 与语言版本链接使用正式域名，再在搜索平台提交站点地图。

Preview 与 Production 应分别检查；本地构建成功不代表已经部署，也不代表已被搜索引擎收录。

实现依据：[Vercel 自定义 404](https://vercel.com/kb/guide/custom-404-page)、[项目配置](https://vercel.com/docs/project-configuration/vercel-json)和[系统环境变量](https://vercel.com/docs/environment-variables/system-environment-variables)。

安装与构建命令固定 pnpm 版本，避免 Vercel 自定义安装命令选用较旧的 pnpm。升级时同时更新 `packageManager` 与 `vercel.json`。依据：[Vercel 包管理器](https://vercel.com/docs/package-managers)。
