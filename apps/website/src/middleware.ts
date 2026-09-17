import { defineMiddleware } from 'astro:middleware';

// Astro development requests use the same locale boundary as the static Vercel routes.
// Production is static: vercel.json, not this middleware, handles unknown URLs there.
export const onRequest = defineMiddleware(async (context, next) => {
  const response = await next();
  const path = context.url.pathname;
  if (/^\/(?:en\/)?404(?:\.html|\/|\/index\.html)?$/.test(path)) {
    return new Response(response.body, {
      status: 404,
      statusText: 'Not Found',
      headers: response.headers,
    });
  }
  if (response.status !== 404) return response;
  const target = /^\/en(?:\/|$)/.test(path) ? '/en/404/' : '/404';
  const page = await context.rewrite(new URL(target, context.url));
  return new Response(page.body, { status: 404, statusText: 'Not Found', headers: page.headers });
});
