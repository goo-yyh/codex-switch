export type Locale = 'zh-CN' | 'en';
export const localeFromPath = (path: string): Locale =>
  /^\/en(?:\/|$)/.test(path) ? 'en' : 'zh-CN';
export const basePath = (path: string) => path.replace(/^\/en(?=\/|$)/, '') || '/';
export function localizedPath(path: string, locale: Locale) {
  const base = basePath(path);
  if (/^\/404(?:\.html|\/|\/index\.html)?$/.test(base))
    return locale === 'en' ? '/en/404/' : '/404.html';
  return locale === 'en' ? `/en${base}` : base;
}
