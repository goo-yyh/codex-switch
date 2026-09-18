import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from release import TARGETS, asset_name, collect, manifest, validate_version


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.output = self.root / 'assets'
        self.output.mkdir()
        for name in ['package.json', 'apps/desktop/package.json', 'apps/website/package.json',
                     'apps/desktop/src-tauri/tauri.conf.json', 'packages/product-info/product.json']:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({'version': '0.1.0'}))
        (self.root / 'Cargo.toml').write_text('[workspace.package]\nversion = "0.1.0"\n')
        (self.root / 'Cargo.lock').write_text(''.join(
            f'[[package]]\nname = "{name}"\nversion = "0.1.0"\n'
            for name in ['codex-switch-core', 'codex-switch-desktop']))

    def test_matching_version(self):
        self.assertEqual(validate_version('v0.1.0', self.root), '0.1.0')

    def test_rejects_invalid_tags_and_version_drift(self):
        for tag in ['main', 'v01.1.0', 'v0.1.0;echo bad', 'v0.1.0+build', 'v0.1.0-01', 'v0.2.0']:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                validate_version(tag, self.root)
        (self.root / 'Cargo.lock').write_text('[[package]]\nname = "codex-switch-desktop"\nversion = "0.0.9"\n')
        with self.assertRaises(ValueError):
            validate_version('v0.1.0', self.root)

    def test_prerelease_version(self):
        for path in self.root.rglob('*'):
            if path.is_file():
                path.write_text(path.read_text().replace('0.1.0', '0.2.0-beta.1'))
        self.assertEqual(validate_version('v0.2.0-beta.1', self.root), '0.2.0-beta.1')

    def test_collect_preserves_architecture_and_hashes_all_three(self):
        for target, (folder, suffix) in TARGETS.items():
            directory = self.root / 'target' / target / 'release' / 'bundle' / folder
            directory.mkdir(parents=True)
            source = directory / f'Codex Switch_0.1.0_{suffix}'
            source.write_bytes(target.encode())
            destination = collect('0.1.0', target, self.output, self.root)
            self.assertEqual(destination.name, asset_name(target))
        manifest(self.output)
        lines = (self.output / 'SHA256SUMS.txt').read_text().splitlines()
        self.assertEqual(len(lines), 3)
        for line in lines:
            digest, name = line.split('  ')
            self.assertEqual(digest, hashlib.sha256((self.output / name).read_bytes()).hexdigest())
        manifest(self.output)  # rerunning manifest is deterministic
        self.assertEqual(lines, (self.output / 'SHA256SUMS.txt').read_text().splitlines())

    def test_rejects_missing_empty_wrong_version_and_ambiguous_installers(self):
        target = 'aarch64-apple-darwin'
        directory = self.root / 'target' / target / 'release/bundle/dmg'
        directory.mkdir(parents=True)
        for names in [[], ['App_0.0.9_aarch64.dmg'],
                      ['App_0.1.0_aarch64.dmg', 'Other_0.1.0_aarch64.dmg']]:
            for path in directory.iterdir():
                path.unlink()
            for name in names:
                (directory / name).write_bytes(b'installer')
            with self.assertRaises(ValueError):
                collect('0.1.0', target, self.output, self.root)
        for path in directory.iterdir():
            path.unlink()
        (directory / 'App_0.1.0_aarch64.dmg').touch()
        with self.assertRaises(ValueError):
            collect('0.1.0', target, self.output, self.root)

    def test_manifest_rejects_missing_extra_and_empty_assets(self):
        with self.assertRaises(ValueError):
            manifest(self.output)
        for target in TARGETS:
            (self.output / asset_name(target)).write_bytes(b'installer')
        extra = self.output / 'unexpected.exe'
        extra.write_bytes(b'wrong')
        with self.assertRaises(ValueError):
            manifest(self.output)
        extra.unlink()
        (self.output / asset_name(next(iter(TARGETS)))).write_bytes(b'')
        with self.assertRaises(ValueError):
            manifest(self.output)


if __name__ == '__main__':
    unittest.main()
