import type { Profile } from '../../api/types';

export function nextProfileName(
  profiles: Profile[],
  base: string,
  presetId: string,
  excludeId = '',
) {
  profiles = profiles.filter((p) => p.id !== excludeId);
  let n = profiles.filter((p) => p.presetId === presetId).length + 1;
  let name = n === 1 ? base : `${base}-${n}`;
  while (profiles.some((p) => p.name.toLowerCase() === name.toLowerCase())) name = `${base}-${++n}`;
  return name;
}

// UI hints mirror the native credential policy; the native layer remains authoritative.
// A per-model endpoint change also invalidates reuse of the shared profile key.
export function canReuseCredential(form: Profile, original?: Profile): boolean {
  const normalizeScope = (endpoint: string, fullUrl = false) => {
    try {
      const u = new URL(endpoint.trim());
      if (fullUrl) return u.href;
      u.pathname = u.pathname
        .replace(/\/+$/, '')
        .replace(/\/(chat\/completions|responses|models)$/, '');
      return u.href.replace(/\/+$/, '');
    } catch {
      return null;
    }
  };
  const scope = normalizeScope(form.endpoint, form.options?.fullUrl);
  return Boolean(
    original &&
    original.presetId === form.presetId &&
    scope &&
    normalizeScope(original.endpoint, original.options?.fullUrl) === scope &&
    Boolean(original.options?.fullUrl) === Boolean(form.options?.fullUrl) &&
    form.models.every((model) => {
      const before = original.options?.modelOverrides?.[model];
      const after = form.options?.modelOverrides?.[model];
      const oldFull = before?.fullUrl ?? Boolean(original.options?.fullUrl);
      const newFull = after?.fullUrl ?? Boolean(form.options?.fullUrl);
      return (
        oldFull === newFull &&
        normalizeScope(before?.endpoint || original.endpoint, oldFull) ===
          normalizeScope(after?.endpoint || form.endpoint, newFull)
      );
    }),
  );
}
