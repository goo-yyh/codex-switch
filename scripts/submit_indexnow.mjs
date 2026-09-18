import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readFileSync, readdirSync } from 'node:fs';

// Opt into the system curl transport when local Node TLS cannot reach the site.
async function request(url, options = {}) {
  if (process.env.INDEXNOW_USE_CURL !== '1') return fetch(url, options);
  const args = [
    '--silent',
    '--show-error',
    '--max-time',
    '30',
    '--request',
    options.method ?? 'GET',
  ];
  for (const [name, value] of Object.entries(options.headers ?? {}))
    args.push('--header', `${name}: ${value}`);
  if (options.body) args.push('--data-binary', options.body);
  args.push('--write-out', '\n%{http_code}\n%header{x-robots-tag}', url);
  let stdout;
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      ({ stdout } = await promisify(execFile)('curl', args, { maxBuffer: 10 * 1024 * 1024 }));
      break;
    } catch (error) {
      if (options.method === 'POST' || attempt === 2) throw error;
    }
  }
  const match = stdout.match(/\n(\d{3})\n([^\n]*)$/);
  if (!match) throw new Error('Invalid curl response.');
  return {
    status: Number(match[1]),
    statusText: '',
    headers: new Headers({ 'x-robots-tag': match[2] }),
    text: async () => stdout.slice(0, match.index),
  };
}

const dist = new URL('../apps/website/dist/', import.meta.url);
const origin = 'https://www.codex-switch.com';
const key = 'dad1031515d24f07b720781d510fe122';
const keyLocation = `${origin}/${key}.txt`;
const dryRun = process.argv.includes('--dry-run');
const inputs = process.argv.slice(2).filter((arg) => arg !== '--dry-run');
const locs = (xml) => [...xml.matchAll(/<loc>(.*?)<\/loc>/g)].map((match) => match[1]);
const sitemapFiles = readdirSync(dist)
  .filter((file) => /^sitemap-\d+\.xml$/.test(file))
  .sort();
if (!sitemapFiles.length)
  throw new Error('Build with PUBLIC_SITE_URL=https://www.codex-switch.com first.');
const sitemapUrls = sitemapFiles.flatMap((file) => locs(readFileSync(new URL(file, dist), 'utf8')));
const urls = [
  ...new Set(inputs.length ? inputs.map((value) => new URL(value, origin).href) : sitemapUrls),
];
if (!urls.length) throw new Error('No URLs to submit.');
for (const value of urls) {
  const url = new URL(value);
  if (
    url.origin !== origin ||
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    !sitemapUrls.includes(value)
  ) {
    throw new Error(`URL must be a canonical production sitemap entry: ${value}`);
  }
}
if (readFileSync(new URL(`${key}.txt`, dist), 'utf8').trim() !== key)
  throw new Error('Built key file mismatch.');
if (
  !readFileSync(new URL('robots.txt', dist), 'utf8').includes(
    `Sitemap: ${origin}/sitemap-index.xml`,
  )
) {
  throw new Error('Build must be indexable production output.');
}
console.log(`${dryRun ? 'Dry run' : 'Submit'}: ${urls.length} URLs\n${urls.join('\n')}`);
if (!dryRun) {
  async function get(url) {
    const response = await request(url, { redirect: 'manual', signal: AbortSignal.timeout(30000) });
    if (response.status !== 200) throw new Error(`${url}: expected 200, got ${response.status}`);
    if (/noindex/i.test(response.headers.get('x-robots-tag') ?? ''))
      throw new Error(`${url}: noindex header`);
    return response.text();
  }
  if ((await get(keyLocation)).trim() !== key)
    throw new Error('Deploy the matching key file before submitting.');
  const liveRobots = await get(`${origin}/robots.txt`);
  if (
    !liveRobots.includes(`Sitemap: ${origin}/sitemap-index.xml`) ||
    /^Disallow:\s*\/\s*$/im.test(liveRobots)
  ) {
    throw new Error('Production robots does not allow indexing.');
  }
  const liveSitemaps = locs(await get(`${origin}/sitemap-index.xml`));
  const liveUrls = new Set();
  for (const sitemap of liveSitemaps) {
    if (new URL(sitemap).origin !== origin) throw new Error('Unexpected sitemap host.');
    for (const url of locs(await get(sitemap))) liveUrls.add(url);
  }
  for (const url of urls) {
    if (!liveUrls.has(url)) throw new Error(`URL is not deployed in the live sitemap: ${url}`);
    const html = await get(url);
    if (/<meta\b[^>]*name=["']robots["'][^>]*content=["'][^"']*noindex/i.test(html))
      throw new Error(`${url}: noindex page`);
    if (!html.includes(`rel="canonical" href="${url}"`))
      throw new Error(`${url}: canonical mismatch`);
  }
  for (let offset = 0; offset < urls.length; offset += 10000) {
    const urlList = urls.slice(offset, offset + 10000);
    const response = await request('https://api.indexnow.org/indexnow', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json; charset=utf-8' },
      body: JSON.stringify({ host: new URL(origin).host, key, keyLocation, urlList }),
      signal: AbortSignal.timeout(30000),
    });
    console.log(
      `IndexNow: ${response.status} ${response.statusText}; ${urlList.length} URLs; ${(await response.text()) || '<empty>'}`,
    );
    if (![200, 202].includes(response.status)) throw new Error('IndexNow submission failed.');
    console.log(
      response.status === 202
        ? 'Received; key validation pending.'
        : 'Received; indexing is not guaranteed.',
    );
  }
}
