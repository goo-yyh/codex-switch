#!/usr/bin/env python3
"""Validate release versions and collect a complete unsigned installer set."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib

ROOT = Path(__file__).resolve().parents[1]
NUMBER = r'(?:0|[1-9][0-9]*)'
IDENT = r'(?:0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)'
TAG = re.compile(rf'v({NUMBER}\.{NUMBER}\.{NUMBER}(?:-{IDENT}(?:\.{IDENT})*)?)')
TARGETS = {
    'aarch64-apple-darwin': ('dmg', 'aarch64.dmg'),
    'x86_64-apple-darwin': ('dmg', 'x64.dmg'),
    'x86_64-pc-windows-msvc': ('nsis', 'x64-setup.exe'),
}


def validate_version(tag, root=ROOT):
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError('Use vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-beta.1 (no build metadata).')
    version = match[1]
    manifests = ['package.json', 'apps/desktop/package.json',
                 'apps/website/package.json', 'apps/desktop/src-tauri/tauri.conf.json',
                 'packages/product-info/product.json']
    for file in manifests:
        actual = json.loads((root / file).read_text(encoding='utf-8'))['version']
        if actual != version:
            raise ValueError(f'{file}: version {actual} does not match {tag}')
    cargo = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    if cargo['workspace']['package']['version'] != version:
        raise ValueError(f'Cargo.toml: workspace version does not match {tag}')
    lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
    for name in ['codex-switch-core', 'codex-switch-desktop']:
        packages = [p for p in lock['package'] if p['name'] == name and 'source' not in p]
        if len(packages) != 1 or packages[0]['version'] != version:
            raise ValueError(f'Cargo.lock: {name} version does not match {tag}')
    return version


def asset_name(target):
    return f'Codex-Switch_{TARGETS[target][1]}'


def collect(version, target, output, root=ROOT):
    folder, suffix = TARGETS[target]
    bundle = root / 'target' / target / 'release' / 'bundle' / folder
    candidates = list(bundle.glob(f'*_{version}_{suffix}'))
    if len(candidates) != 1 or candidates[0].stat().st_size == 0:
        raise ValueError(f'Expected exactly one nonempty {version}/{target} installer in {bundle}')
    output.mkdir(parents=True, exist_ok=True)
    destination = output / asset_name(target)
    shutil.copyfile(candidates[0], destination)
    return destination


def manifest(output):
    expected = {asset_name(target) for target in TARGETS}
    actual = {p.name for p in output.iterdir() if p.name != 'SHA256SUMS.txt'}
    if actual != expected:
        raise ValueError(f'Incomplete/unexpected assets: missing={expected-actual}, extra={actual-expected}')
    lines = []
    for name in sorted(expected):
        file = output / name
        if not file.is_file() or file.is_symlink() or file.stat().st_size == 0:
            raise ValueError(f'Invalid or empty installer: {name}')
        with file.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        lines.append(f'{digest}  {name}\n')
    (output / 'SHA256SUMS.txt').write_text(''.join(lines), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['check', 'collect', 'manifest'])
    parser.add_argument('--tag', required=True)
    parser.add_argument('--target', choices=TARGETS)
    parser.add_argument('--output', type=Path, default=ROOT / 'release-assets')
    args = parser.parse_args()
    version = validate_version(args.tag)
    if args.command == 'collect':
        if not args.target:
            parser.error('collect requires --target')
        print(collect(version, args.target, args.output))
    elif args.command == 'manifest':
        manifest(args.output)
    print(f'PASS: {args.command} {args.tag}')


if __name__ == '__main__':
    main()
