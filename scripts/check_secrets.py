#!/usr/bin/env python3
"""Check Git candidates and optional local build outputs without printing credentials."""
from pathlib import Path
import argparse
import re
import shlex
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--artifacts', action='store_true')
args = parser.parse_args()
keys = []
env_file = ROOT / '.env'
if env_file.exists():
    for line in env_file.read_text().splitlines():
        if not line.strip() or line.lstrip().startswith('#') or '=' not in line:
            continue
        name, value = line.split('=', 1)
        if name.strip().removeprefix('export ') in {'qianwen_key','minimax_key','zhipu_key','kimi_key','deepseek_key'}:
            try:
                parsed = shlex.split(value, comments=True)
            except ValueError:
                sys.exit('Cannot parse local credential file; no values displayed.')
            if parsed and len(parsed[0]) >= 8:
                keys.append(parsed[0].encode())
raw = subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'], cwd=ROOT)
paths = {ROOT / name.decode() for name in raw.split(b'\0') if name}
issues = set()
for p in paths:
    if p.name.startswith('.env') and p.name != '.env.example':
        issues.add((str(p.relative_to(ROOT)), 'credential file is a Git candidate'))
if args.artifacts:
    for base in ['apps/desktop/dist', 'apps/website/dist', 'target/release/bundle']:
        folder = ROOT / base
        if folder.exists():
            paths.update(p for p in folder.rglob('*') if p.is_file() and not p.is_symlink())
pattern = re.compile(rb'\b(?:sk-[A-Za-z0-9_-]{24,}|AKIA[A-Z0-9]{16})\b')
for p in sorted(paths):
    if not p.is_file() or p.is_symlink():
        continue
    tail = b''
    with p.open('rb') as file:
        while chunk := file.read(1024 * 1024):
            data = tail + chunk
            if any(key in data for key in keys) or pattern.search(data):
                issues.add((str(p.relative_to(ROOT)), 'potential credential content'))
                break
            tail = data[-max([512, *(len(key) for key in keys)]):]
    if p.name == '.env.example':
        for line in p.read_text().splitlines():
            if line.strip() and not line.lstrip().startswith('#') and '=' in line and line.split('=',1)[1].strip():
                issues.add((str(p.relative_to(ROOT)), 'example values must be empty'))
if issues:
    for path, reason in sorted(issues):
        print(f'FAIL: {path}: {reason}')
    sys.exit(1)
print(f'PASS: {len(paths)} source/artifact files checked; credential values were not displayed.')
