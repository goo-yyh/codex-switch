#!/usr/bin/env python3
"""Verify the pinned upstream files/excerpts without network or credentials."""
import hashlib
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
base = root / 'crates/cc-switch-codex'
manifest = json.loads((base / 'upstream-files.json').read_text())
for item in manifest['files']:
    content = (base / 'src' / item['path']).read_bytes()
    assert hashlib.sha256(content).hexdigest() == item['sha256'], item['path']
for item in manifest['fragments']:
    content = (base / 'src' / item['path']).read_text()
    start = content.index(item['start'])
    fragment = content[start:content.index(item['end'], start)].rstrip()
    assert hashlib.sha256(fragment.encode()).hexdigest() == item['sha256'], item['start']
registry = root / 'packages/provider-registry'
source = json.loads((registry / 'upstream.json').read_text())
assert source['commit'] == manifest['commit']
assert hashlib.sha256((registry / 'providers.json').read_bytes()).hexdigest() == source['registrySha256']
assert 'Copyright (c) 2025 Jason Young' in (base / 'LICENSE').read_text()
print(f"PASS: {len(manifest['files'])} pinned files, {len(manifest['fragments'])} excerpts, registry and license; CC Switch {manifest['commit'][:12]}")
