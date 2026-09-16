import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const output = mkdtempSync(join(tmpdir(), 'codex-switch-icons-'));
try {
  const pnpmEntry = process.env.npm_execpath;
  if (!pnpmEntry) throw new Error('Run this export using pnpm icons.');
  const result = spawnSync(
    process.execPath,
    [
      pnpmEntry,
      '--filter',
      '@codex-switch/desktop',
      'tauri',
      'icon',
      join(root, 'packages/brand/icon.png'),
      '--output',
      output,
    ],
    { cwd: root, stdio: 'inherit' },
  );
  if (result.error || result.status !== 0) throw new Error('Icon export failed.');
  const desktop = join(root, 'apps/desktop/src-tauri/icons');
  mkdirSync(desktop, { recursive: true });
  // Keep only desktop formats; this project has no mobile app.
  for (const entry of readdirSync(output, { withFileTypes: true })) {
    if (entry.isFile() && /\.(png|ico|icns)$/.test(entry.name)) {
      copyFileSync(join(output, entry.name), join(desktop, entry.name));
    }
  }
  for (const target of ['packages/brand/mark.png', 'apps/website/public/mark.png']) {
    copyFileSync(join(output, '128x128@2x.png'), join(root, target));
  }
} finally {
  rmSync(output, { recursive: true, force: true });
}
