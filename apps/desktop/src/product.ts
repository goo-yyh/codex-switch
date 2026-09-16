import product from '../../../packages/product-info/product.json';
export { product };
export function documentationUrl(): string {
  try {
    const url = new URL(import.meta.env.VITE_PUBLIC_DOCS_URL || product.sourceDocs);
    if (url.protocol === 'https:' && !url.username && !url.password) return url.href;
  } catch {
    /* Use the bundled source documentation until a site is configured. */
  }
  return product.sourceDocs;
}
