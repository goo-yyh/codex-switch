# SEO 与 IndexNow 检查（2026-09-18）

正式主域名：`https://www.codex-switch.com`。裸域名 HTTP 308 跳转到 www。

参考本地 `codex_doc_cn` 的 `src/lib/seo.ts`、`src/lib/seo/page-seo.ts` 和 `scripts/submit_indexnow.mjs`，逐项核对页面元信息、语言配对、结构化数据、抓取策略和站点地图。沿用本站现有双语 Astro 实现。

## 结果

- 18 个可索引页面：9 个中文、9 个英文。标题及描述唯一，单个 H1，HTML 语言正确。
- 每页 canonical 指向正式域名自身；zh-CN、en、x-default 语言链接正确，语言切换指向对应页面。
- Open Graph、Twitter 分享元信息及图片可用，图片实际尺寸为 1100 × 850。
- 修正中英文首页被标为 TechArticle/article 的问题，改用 WebPage/website；首页保留 SoftwareApplication，文档保留 TechArticle 和 BreadcrumbList。
- robots 允许正式站抓取并声明 sitemap-index.xml；sitemap 精确覆盖 18 个 canonical 页面，无 404 或旧重定向路径。
- 两种语言的 404 为 noindex，无 canonical、hreflang 或 JSON-LD。线上不存在的中英文路径返回 HTTP 404；6 个旧路径返回 HTTP 308。
- 新增根路径 IndexNow UTF-8 密钥文件与 `pnpm indexnow:submit`。默认提交构建站点地图，也支持指定其中的更新 URL。拒绝外部域名和非站点地图 URL。
- 提交前核对线上密钥内容、robots、站点地图、页面 HTTP 200、索引许可和 canonical。支持 dry-run；本机 TLS 不稳定时可选择系统 curl 传输。

## 验证证据

- `PUBLIC_SITE_URL=https://www.codex-switch.com pnpm build:website`：通过。
- 网站 Astro check：0 errors / 0 warnings / 0 hints。
- `pnpm check:seo -- --require-site --expect-indexable`：20 页通过（含 2 个 404）。
- `pnpm check:links`：26 个构建 HTML 的本地链接及锚点通过（含 6 个重定向）。
- `pnpm check:deployment`：33 项通过，包括预览索引策略、模型目录和双语 404 路由。
- IndexNow dry-run：18 个 URL；外部域名及 `/404/` 拒绝提交。
- 正式提交：`INDEXNOW_USE_CURL=1 pnpm indexnow:submit`，18 个 URL，HTTP **202 Accepted**，响应体为空。表示已接收、密钥验证待完成，不代表收录；没有重复发送以追求 200。
- Vercel Production：`dpl_9u27WFvHtJNELpUexpXyJJf8juDE`，READY，已绑定正式 www 域名。
- 上线后重新抓取 31 个资源，使用 SEO 校验器核查线上 HTML、robots、站点地图和图片；另查两个不存在的路径，全部通过。连接偶发 TLS 中断，读取重试后完成。
- `git diff --check`、脚本语法检查及凭据扫描通过。

## 范围

以上证明技术 SEO 和抓取入口可用，不代表 Bing 已经收录、获得排名或通过真实用户 Core Web Vitals。未进行 Lighthouse/真实用户性能测试，未读取 Bing Webmaster Tools 的收录数据。本次检查完成时已通过 Vercel CLI 发布网站；Git 提交与推送作为后续独立步骤执行。

协议依据：https://www.indexnow.org/documentation
