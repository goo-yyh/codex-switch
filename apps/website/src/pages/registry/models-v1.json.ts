import type { APIRoute } from 'astro';
import { createHash } from 'node:crypto';
import models from '../../../../../packages/provider-registry/models.json';
import providers from '../../../../../packages/provider-registry/providers.json';
import registryVersion from '../../../../../packages/provider-registry/version.json';

export const prerender = true;

const { version } = registryVersion;
if (!Number.isInteger(version) || version <= 0 || version > 0xffffffff) {
  throw new Error('Registry version must be a positive 32-bit integer');
}

// Build from the same public presets shipped with the app, never local user data.
// A content revision stays stable across deployments when the registry is unchanged.
const content = { schemaVersion: 1, version, models, providers };
const revision = createHash('sha256').update(JSON.stringify(content)).digest('hex');

export const GET: APIRoute = () =>
  new Response(JSON.stringify({ ...content, revision }, null, 2) + '\n', {
    headers: { 'Content-Type': 'application/json; charset=utf-8' },
  });
