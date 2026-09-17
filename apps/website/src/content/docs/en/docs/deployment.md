---
title: Deploy the bilingual website to Vercel
description: Deploy the static Codex Switch website from the repository root, configure production SEO, and serve Chinese or English pages with real HTTP 404 responses.
---

The repository includes a root `vercel.json`. Vercel installs frontend dependencies and builds only the website, without the desktop app, Rust or model API keys.

## Import the project

1. Import the `goo-yyh/codex-switch` repository into Vercel.
2. Keep **Root Directory at the repository root `.`**, not `apps/website`. The build uses shared workspace packages and the root lockfile.
3. Choose **Other** as the Framework Preset. `vercel.json` supplies the build settings.
4. Enable **Automatically expose System Environment Variables** in the project environment settings, then deploy.

| Setting          | Repository value                                   |
| ---------------- | -------------------------------------------------- |
| Node.js          | `22.x`, from the root `package.json`               |
| Package manager  | `pnpm@10.26.0`, from `packageManager`              |
| Install Command  | `npx --yes pnpm@10.26.0 install --frozen-lockfile` |
| Build Command    | `npx --yes pnpm@10.26.0 build:website`             |
| Output Directory | `apps/website/dist`                                |

This is a static Astro + Starlight site. It needs neither a Serverless Function nor a Vercel SSR adapter. Do not add a catch-all SPA rewrite to the home page: it would turn missing pages into successful responses.

## Production domain and previews

Set `PUBLIC_SITE_URL=https://your-production-domain` in both **Production** and **Preview** environments. Bind your custom domain and rebuild to apply it.

| Variable                           | Behavior                                                                                                |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `PUBLIC_SITE_URL`                  | Preferred production HTTPS origin, without a subpath, query or credentials.                             |
| `VERCEL_PROJECT_PRODUCTION_URL`    | Stable Vercel production domain used when no explicit origin is set; `https://` is added automatically. |
| `VERCEL_ENV` / `VERCEL_TARGET_ENV` | Distinguishes production, preview and custom environments. Only production can be indexed.              |
| `PUBLIC_SITE_INDEXABLE=false`      | Also disables indexing in production. Setting it to `true` cannot enable indexing in Preview.           |

The changing `VERCEL_URL` is never used as canonical. Preview pages stay `noindex, nofollow` and robots disallows crawling, even when a production domain is available. Canonical and alternate links use the stable production domain.

A Vercel production build fails with a configuration message when neither an explicit nor a system-provided production domain exists. Local builds without a domain still work and remain unindexable. Domain or SEO environment changes require a new build and deployment.

## Localized 404 responses

Existing pages and static assets are resolved before missing-page fallbacks:

| Example request                      | Result                                                            |
| ------------------------------------ | ----------------------------------------------------------------- |
| `/docs/not-found/`, `/missing`       | Chinese page, HTTP 404                                            |
| `/en/docs/not-found/`, `/en/missing` | English page, HTTP 404                                            |
| `/english/missing`, `/fr/missing`    | Default Chinese page; similar prefixes are not treated as English |
| `/404.html`, `/en/404/`              | Corresponding error page, HTTP 404                                |

Error pages include home, documentation and language-switch links. They do not redirect to the home page, and the requested URL stays in the address bar. They omit canonical links, language alternates and structured data, and stay out of the sitemap and site search.

## Local checks

```sh
pnpm build:website
pnpm check:links
pnpm check:seo
pnpm check:deployment
pnpm preview:website
```

The final command starts a static routing preview at `http://127.0.0.1:4324`. Open `/en/docs/not-found/` and `/docs/not-found/` to inspect both 404s. The ordinary `pnpm website` development server also preserves the requested language; production static routing is defined by `vercel.json`.

`check:deployment` uses built files and the actual `vercel.json` in local HTTP checks for normal pages, assets, both 404 languages, similar prefixes, GET / HEAD and indexing rules. It does not emulate or validate Vercel’s cloud runtime.

## After deployment

On the actual deployment, verify that both home pages load and normal pages return HTTP 200. Missing Chinese and English paths must return HTTP 404 in the correct language. Check `/robots.txt`, `/sitemap-index.xml`, canonical URLs and language alternates against the production domain before submitting the sitemap to search platforms.

Check Preview and Production separately. A passing local build does not mean the site is deployed or indexed.

References: [Vercel custom 404 pages](https://vercel.com/kb/guide/custom-404-page), [project configuration](https://vercel.com/docs/project-configuration/vercel-json) and [system environment variables](https://vercel.com/docs/environment-variables/system-environment-variables).

Install and build commands pin pnpm explicitly to avoid an older default with custom install commands. Keep `packageManager` and `vercel.json` synchronized when upgrading. See [Vercel package managers](https://vercel.com/docs/package-managers).
