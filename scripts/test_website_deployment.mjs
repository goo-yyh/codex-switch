import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolveSiteConfig } from '../apps/website/site-config.mjs';
import { createPreviewServer } from './preview_website.mjs';

const config = JSON.parse(readFileSync(new URL('../vercel.json', import.meta.url), 'utf8'));
let server, origin;
before(async () => {
  server = createPreviewServer();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
});
after(() => new Promise((resolve) => server.close(resolve)));

test('model registry serves JSON with matching candidates, capabilities and content revision', async () => {
  const response = await fetch(origin + '/registry/models-v1.json');
  assert.equal(response.status, 200);
  assert.equal(response.redirected, false);
  assert.match(response.headers.get('content-type'), /^application\/json/);
  const { revision, ...content } = await response.json();
  assert.equal(content.schemaVersion, 1);
  for (const name of ['models', 'providers']) {
    const source = JSON.parse(
      readFileSync(new URL(`../packages/provider-registry/${name}.json`, import.meta.url), 'utf8'),
    );
    assert.deepEqual(content[name], source);
  }
  assert.equal(revision, createHash('sha256').update(JSON.stringify(content)).digest('hex'));
  for (const [id, models] of Object.entries(content.models)) {
    assert.ok(models.length > 0 && models.length <= 5, id);
    const preset = content.providers.find((provider) => provider.id === id);
    assert.ok(preset, id);
    for (const model of models) {
      assert.ok(preset.options.modelOverrides[model]?.contextWindow > 0, `${id}/${model}`);
    }
  }
});

test('website deployment uses the repository root and only builds the static website', () => {
  assert.equal(config.buildCommand, 'npx --yes pnpm@10.26.0 build:website');
  assert.equal(config.outputDirectory, 'apps/website/dist');
  assert.equal(config.installCommand, 'npx --yes pnpm@10.26.0 install --frozen-lockfile');
  assert.equal(config.framework, null);
  const fallback = config.routes.findIndex((r) => r.src === '^/en(?:/.*)?$');
  assert.ok(config.routes.findIndex((r) => r.handle === 'filesystem') < fallback);
});
test('canonical origin uses an explicit domain or the stable Vercel production domain', () => {
  assert.deepEqual(resolveSiteConfig({}), { site: undefined, indexable: false });
  assert.deepEqual(
    resolveSiteConfig({
      VERCEL: '1',
      VERCEL_ENV: 'production',
      VERCEL_PROJECT_PRODUCTION_URL: 'project.vercel.app',
    }),
    { site: 'https://project.vercel.app', indexable: true },
  );
  assert.deepEqual(
    resolveSiteConfig({
      VERCEL_ENV: 'production',
      PUBLIC_SITE_URL: 'https://docs.example.test/',
      VERCEL_PROJECT_PRODUCTION_URL: 'project.vercel.app',
    }),
    { site: 'https://docs.example.test', indexable: true },
  );
  assert.throws(
    () => resolveSiteConfig({ VERCEL_ENV: 'production', VERCEL_URL: 'random-preview.vercel.app' }),
    /PUBLIC_SITE_URL/,
  );
  assert.equal(resolveSiteConfig({ VERCEL_URL: 'random-preview.vercel.app' }).site, undefined);
  for (const PUBLIC_SITE_URL of [
    'http://example.test',
    'https://example.test/path',
    'https://user:pass@example.test',
    'https://example.test/?a=1',
  ]) {
    assert.throws(() => resolveSiteConfig({ PUBLIC_SITE_URL }));
  }
});
test('preview, development, custom and unclassified Vercel builds cannot be made indexable', () => {
  for (const extra of [
    { VERCEL_ENV: 'preview' },
    { VERCEL_ENV: 'development' },
    { VERCEL_ENV: 'production', VERCEL_TARGET_ENV: 'staging' },
    { VERCEL: '1' },
  ]) {
    assert.equal(
      resolveSiteConfig({
        ...extra,
        PUBLIC_SITE_URL: 'https://example.test',
        PUBLIC_SITE_INDEXABLE: 'true',
      }).indexable,
      false,
    );
  }
  assert.equal(
    resolveSiteConfig({
      VERCEL_ENV: 'production',
      PUBLIC_SITE_URL: 'https://example.test',
      PUBLIC_SITE_INDEXABLE: 'false',
    }).indexable,
    false,
  );
});
for (const [path, destination] of [
  ['/docs/overview/', '/'],
  ['/en/docs/overview/', '/en/'],
  ['/docs/relay/', '/docs/providers/'],
  ['/en/docs/relay/', '/en/docs/providers/'],
  ['/docs/launch/', '/docs/switch/'],
  ['/en/docs/launch/', '/en/docs/switch/'],
]) {
  test(`old documentation redirects to its replacement: ${path}`, async () => {
    const response = await fetch(origin + path, { redirect: 'manual' });
    assert.equal(response.status, 308);
    assert.equal(response.headers.get('location'), destination);
  });
}
for (const path of [
  '/',
  '/en/',
  '/docs/quickstart/',
  '/en/docs/quickstart/',
  '/docs/changelog/',
  '/en/docs/changelog/',
  '/mark.png',
  '/robots.txt',
]) {
  test(`existing file stays available: ${path}`, async () => {
    const response = await fetch(origin + path);
    assert.equal(response.status, 200);
    if (path.endsWith('/')) {
      const body = await response.text();
      assert.match(
        body,
        path.startsWith('/en/') ? /<html[^>]+lang="en"/ : /<html[^>]+lang="zh-CN"/,
      );
      if (path === '/' || path === '/en/') {
        assert.match(body, /class="docs-header"/);
        assert.match(body, /<docs-theme-toggle/);
        assert.doesNotMatch(body, /class="hero section-wrap"/);
      }
    }
  });
}
for (const [path, language] of [
  ['/download/', 'zh-CN'],
  ['/en/download/', 'en'],
  ['/missing', 'zh-CN'],
  ['/docs/missing/nested/', 'zh-CN'],
  ['/missing.png', 'zh-CN'],
  ['/en/missing', 'en'],
  ['/en/docs/missing/?from=test', 'en'],
  ['/en/missing.png', 'en'],
  ['/english/missing', 'zh-CN'],
  ['/enough/missing', 'zh-CN'],
  ['/fr/missing', 'zh-CN'],
  ['/404.html', 'zh-CN'],
  ['/404/', 'zh-CN'],
  ['/en/404/', 'en'],
  ['/en/404/index.html', 'en'],
]) {
  test(`missing page preserves status and locale: ${path}`, async () => {
    const response = await fetch(origin + path);
    assert.equal(response.status, 404);
    assert.equal(response.redirected, false);
    const body = await response.text();
    assert.match(body, new RegExp(`<html[^>]+lang="${language}"`));
    assert.match(body, /name="robots" content="noindex, nofollow"/);
    assert.doesNotMatch(
      body,
      /rel="canonical"|rel="alternate"|application\/ld\+json|data-pagefind-body/,
    );
    assert.match(body, language === 'en' ? /404 · Page not found/ : /404 · 页面未找到/);
    assert.ok(
      body.includes(language === 'en' ? 'href="/en/docs/quickstart/"' : 'href="/docs/quickstart/"'),
    );
    assert.equal((await fetch(origin + path, { method: 'HEAD' })).status, 404);
  });
}
