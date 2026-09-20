// Mirror immutable, checksum-pinned release assets into the website's static CDN output.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, rename, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const release = JSON.parse(await readFile(new URL('packages/product-info/downloads.json', root)));
if (!/^\d+\.\d+\.\d+$/.test(release.version)) throw new Error('Invalid download version');
const expected = [
  'Codex-Switch_aarch64.dmg',
  'Codex-Switch_x64.dmg',
  'Codex-Switch_x64-setup.exe',
  'SHA256SUMS.txt',
];
if (JSON.stringify(Object.keys(release.assets).sort()) !== JSON.stringify(expected.sort())) {
  throw new Error('Incomplete release asset manifest');
}
const directory = new URL(`apps/website/public/downloads/v${release.version}/`, root);
await mkdir(directory, { recursive: true });
for (const [name, hash] of Object.entries(release.assets)) {
  if (!/^[a-f0-9]{64}$/.test(hash)) throw new Error(`Invalid checksum: ${name}`);
  const destination = new URL(name, directory);
  const valid = async (file) => {
    try {
      return (
        createHash('sha256')
          .update(await readFile(file))
          .digest('hex') === hash
      );
    } catch (error) {
      if (error.code === 'ENOENT') return false;
      throw error;
    }
  };
  if (!(await valid(destination))) {
    const temporary = new URL(`${name}.partial`, directory);
    try {
      execFileSync(
        'curl',
        [
          '--fail',
          '--location',
          '--silent',
          '--show-error',
          '--retry',
          '3',
          '--connect-timeout',
          '20',
          '--max-time',
          '180',
          '--output',
          fileURLToPath(temporary),
          `https://github.com/goo-yyh/codex-switch/releases/download/v${release.version}/${name}`,
        ],
        { stdio: 'inherit' },
      );
      if (!(await valid(temporary))) throw new Error(`Release checksum mismatch: ${name}`);
      await rename(temporary, destination);
    } finally {
      await rm(temporary, { force: true });
    }
  }
  console.log(`Verified CDN download: v${release.version}/${name}`);
}
