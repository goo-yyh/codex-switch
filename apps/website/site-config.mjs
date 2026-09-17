/** Shared by Astro config, page metadata and robots; never use a preview hostname as canonical. */
export function resolveSiteConfig(env = process.env) {
  const source =
    env.PUBLIC_SITE_URL?.trim() ||
    (env.VERCEL_PROJECT_PRODUCTION_URL
      ? `https://${env.VERCEL_PROJECT_PRODUCTION_URL}`
      : undefined);
  let site;
  if (source) {
    const url = new URL(source);
    if (
      url.protocol !== 'https:' ||
      url.username ||
      url.password ||
      url.pathname !== '/' ||
      url.search ||
      url.hash
    ) {
      throw new Error(
        'PUBLIC_SITE_URL must be an HTTPS origin without credentials, paths, queries or fragments.',
      );
    }
    site = url.origin;
  }
  if (env.VERCEL_ENV === 'production' && !site) {
    throw new Error(
      'Set PUBLIC_SITE_URL or enable Vercel System Environment Variables before a production build.',
    );
  }
  const onVercel = env.VERCEL === '1' || Boolean(env.VERCEL_ENV) || Boolean(env.VERCEL_TARGET_ENV);
  const production =
    env.VERCEL_ENV === 'production' &&
    (!env.VERCEL_TARGET_ENV || env.VERCEL_TARGET_ENV === 'production');
  const indexable =
    Boolean(site) && env.PUBLIC_SITE_INDEXABLE !== 'false' && (!onVercel || production);
  return { site, indexable };
}
export const siteConfig = resolveSiteConfig();
