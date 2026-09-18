import product from '../../../packages/product-info/product.json';
import type { Locale } from './i18n';
export { product };
export function documentationUrl(locale: Locale = 'zh-CN'): string {
  const source =
    locale === 'en'
      ? product.sourceDocs.replace('/content/docs/docs/', '/content/docs/en/docs/')
      : product.sourceDocs;
  try {
    if (!import.meta.env.VITE_PUBLIC_DOCS_URL) return source;
    const url = new URL(import.meta.env.VITE_PUBLIC_DOCS_URL);
    if (url.protocol === 'https:' && !url.username && !url.password) {
      if (url.href === product.sourceDocs) return source;
      const path = url.pathname.replace(/^\/en(?=\/|$)/, '') || '/';
      url.pathname = locale === 'en' ? `/en${path}` : path;
      return url.href;
    }
  } catch {
    /* Use the bundled source documentation until a site is configured. */
  }
  return source;
}
