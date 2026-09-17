---
title: Development, testing and website deployment
description: Preview Codex Switch safely, run offline checks, test model summaries explicitly, and build the bilingual documentation site with SEO validation.
---

## Repository layout

```text
apps/desktop/             React + Vite desktop interface
apps/desktop/src-tauri/   Tauri native shell and platform integration
apps/website/             Astro + Starlight website and docs
crates/core/              Configuration transactions, routing, adapters, SQLite
crates/cc-switch-codex/   Pinned CC Switch code, tests and license
packages/                Shared design tokens, product info and model presets
docs/                    Design notes, source research and historical test reports
```

The desktop UI separates `api/`, `hooks/`, shared `components/`, and `features/profiles/` / `features/settings/`. The native shell separates `commands.rs`, `service.rs`, `state.rs`, and `tray/`; `main.rs` only assembles the app. The core gateway separates forwarding, history replay, HTTP limits, and explicit probes. Bilingual download pages share one `DownloadCards.astro` component.

See the [architecture and maintenance guide](https://github.com/goo-yyh/codex-switch/blob/main/docs/development/architecture.md) for call paths, credential rollback, legacy migration, and native menu constraints. `pnpm test` includes native coordinator tests alongside UI, core, and protocol tests.

## Preview and build

See [installation](/en/docs/install/) for platform requirements.

```sh
pnpm install --frozen-lockfile
pnpm dev       # Desktop browser preview; default port 1420
pnpm website   # Website and docs; default port 4321
```

Run these servers in separate terminals. `pnpm dev` uses page-local mock data, without model calls, Codex configuration changes or credential-store access. Refreshing resets the examples. `pnpm desktop` starts the real native app; it accesses local settings and credentials and changes configuration when enabled.

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

Offline tests use temporary directories, in-memory databases, mock credentials and synthetic SSE, without launching Codex. CI does not run real provider tests or require model keys. `check:upstream` verifies reused files and extracted fragments; see [credits](/en/docs/open-source/).

## Minimal summary test

Create a local `.env` from `.env.example` and fill the key for the provider you explicitly want to test:

```sh
# One provider's default model
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke deepseek
# All five preset defaults
cargo run -p codex-switch-core --bin provider-compact-check -- --summary-smoke --all
```

Each provider receives one ordinary streaming Responses request with about 128 KB of synthetic history, a 2,048-token output limit and a 120-second timeout. Checks cover successful HTTP status, completion, nonempty text, streaming consistency and a summary shorter than the input.

This does not launch Codex, access real sessions or configuration, call remote compact, or verify factual completeness and continuation quality. It uses test presets and `.env`, **not saved desktop app configurations**. See [recorded results](/en/docs/compatibility/#ordinary-summary-check).

## Other real-service tests

| Command                             | Scope                                                                                                                                                      |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm test:providers deepseek`      | Up to four synthetic requests per provider: text, streaming, mock tools and tool results; 256 output tokens and a 60-second timeout per request.           |
| `pnpm test:providers:summary --all` | Up to three ordinary requests per provider: history recall, summary generation and continuation; 2,048 output tokens and a 120-second timeout per request. |
| `pnpm test:providers:compact --all` | Separate remote compaction protocol checks. Ordinary summary success is not a substitute.                                                                  |

These commands call real services and may incur API fees. They do not retry failures on another domain or execute returned tools. Do not `source .env`, pass keys on the command line or upload raw responses. Normal builds and offline tests do not read `.env` to send requests.

## Maintain docs and screenshots

Chinese docs are in `apps/website/src/content/docs/docs/`; English counterparts are in `apps/website/src/content/docs/en/docs/`, with matching filenames. Update both languages together. Navigation lives in `apps/website/astro.config.mjs` and screenshots in `apps/website/public/screenshots/`.

Capture the actual browser preview with fictional credentials and explicit example configurations. Do not reconstruct screenshots or use real keys. See the [capture record](https://github.com/goo-yyh/codex-switch/blob/main/docs/design/screenshots.md). Screenshots demonstrate the interface, not native or provider validation.

## Release information and documentation URLs

`packages/product-info/product.json` holds version, channel, repository, download URLs and SHA-256 checksums. Installer buttons appear only when both a URL and checksum exist. Cargo, package and Tauri versions must also be synchronized before release. There are currently no public installer URLs.

| Build variable          | Purpose                                                                                               |
| ----------------------- | ----------------------------------------------------------------------------------------------------- |
| `PUBLIC_SITE_URL`       | Public HTTPS site origin, used for canonical URLs, language alternates, social images and sitemaps.   |
| `PUBLIC_SITE_INDEXABLE` | Set to `false` for a preview deployment: pages use `noindex, nofollow` and robots disallows crawling. |
| `VITE_PUBLIC_DOCS_URL`  | Independent HTTPS documentation URL used by the app; otherwise it opens source documentation.         |

These are public build values, not credentials. Local builds without `PUBLIC_SITE_URL` use `noindex` and omit absolute SEO URLs. Vercel can use its stable production-domain system variable as a fallback; Preview always disables indexing. The site never invents a production domain. Set your real domain in the hosting environment before building:

```sh
PUBLIC_SITE_URL=https://your-domain.example pnpm --filter @codex-switch/website build
pnpm check:links
pnpm check:seo -- --require-site
```

Replace the example domain before use. Chinese remains the default at `/`; English uses `/en/`. Language switches keep the equivalent page. Titles, descriptions, canonical URLs, `hreflang` links, Open Graph and structured data share one SEO component. `robots.txt` and the sitemap are generated during the build. The SEO check audits every generated page, both language versions and sitemap coverage.

`pnpm build` writes the static site to `apps/website/dist/`. A successful local build does not mean the site is deployed or indexed.

For Vercel, see the [deployment guide](/en/docs/deployment/): repository root settings, production-domain fallback, automatic Preview noindex and localized HTTP 404 routing.
