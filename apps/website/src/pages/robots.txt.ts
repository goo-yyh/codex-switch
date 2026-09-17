import type { APIRoute } from 'astro';
import { siteConfig } from '../../site-config.mjs';
export const GET: APIRoute = ({ site }) =>
  new Response(
    site && siteConfig.indexable
      ? `User-agent: *\nAllow: /\nSitemap: ${new URL('/sitemap-index.xml', site).href}\n`
      : 'User-agent: *\nDisallow: /\n',
    { headers: { 'Content-Type': 'text/plain; charset=utf-8' } },
  );
