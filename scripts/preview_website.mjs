import { createServer } from 'node:http';
import { readFileSync, statSync } from 'node:fs';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const config = JSON.parse(readFileSync(resolve(root, 'vercel.json'), 'utf8'));
const dist = resolve(root, config.outputDirectory);
const types = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.xml': 'application/xml',
  '.txt': 'text/plain; charset=utf-8',
  '.png': 'image/png',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.wasm': 'application/wasm',
  '.woff2': 'font/woff2',
};
function fileAt(path) {
  const file = resolve(dist, '.' + path);
  if (file !== dist && !file.startsWith(dist + sep)) return;
  try {
    if (statSync(file).isFile()) return file;
  } catch {
    /* Not a built file. */
  }
}
function builtFile(path) {
  return fileAt(path) || fileAt(path.replace(/\/$/, '') + '/index.html');
}
/** Local evaluator for this project's static Vercel routes, not a Vercel runtime emulator. */
export function resolveRequest(path) {
  for (const route of config.routes) {
    if (route.handle === 'filesystem') {
      const file = builtFile(path);
      if (file) return { status: 200, file };
    } else if (new RegExp(`^(?:${route.src})$`).test(path)) {
      if (route.headers?.Location) {
        return { status: route.status, location: route.headers.Location };
      }
      const file = fileAt(route.dest);
      if (!file) throw new Error(`Build the website first; missing ${route.dest}`);
      return { status: route.status, file };
    }
  }
  throw new Error('No static fallback route configured.');
}
export function createPreviewServer() {
  return createServer((req, res) => {
    if (req.method !== 'GET' && req.method !== 'HEAD') {
      res.writeHead(405, { Allow: 'GET, HEAD' });
      res.end();
      return;
    }
    try {
      const path = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
      const { file, status, location } = resolveRequest(path);
      if (location) {
        res.writeHead(status, { Location: location });
        res.end();
        return;
      }
      const headers = { 'Content-Type': types[extname(file)] || 'application/octet-stream' };
      if (status === 404) headers['X-Robots-Tag'] = 'noindex';
      res.writeHead(status, headers);
      res.end(req.method === 'HEAD' ? undefined : readFileSync(file));
    } catch {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('Unable to serve the static build. Run pnpm build:website first.');
    }
  });
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const port = Number(process.env.PORT || 4324);
  createPreviewServer().listen(port, '127.0.0.1', () =>
    console.log(`Static routing preview: http://127.0.0.1:${port}`),
  );
}
