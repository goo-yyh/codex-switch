#!/usr/bin/env python3
"""Validate generated local navigation and fragments without a browser or network."""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import urlsplit, unquote
import sys

ROOT = Path(__file__).resolve().parents[1] / 'apps/website/dist'
class Page(HTMLParser):
    def __init__(self, path):
        super().__init__()
        self.links = []
        self.ids = set()
        self.feed(path.read_text())
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.add(attrs['id'])
        if tag == 'a' and attrs.get('href'):
            self.links.append(attrs['href'])

pages = {p: Page(p) for p in ROOT.rglob('*.html')}
if not pages:
    sys.exit('Build the website before checking links.')
errors = []
for file, page in pages.items():
    for href in page.links:
        url = urlsplit(href)
        if url.scheme or url.netloc:
            continue
        if url.path.startswith('/'):
            target = ROOT / unquote(url.path).lstrip('/')
        elif url.path:
            target = file.parent / unquote(url.path)
        else:
            target = file
        if target.is_dir():
            target /= 'index.html'
        if not target.exists():
            errors.append(f'{file.relative_to(ROOT)}: missing {href}')
        elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
            errors.append(f'{file.relative_to(ROOT)}: missing fragment {href}')
if errors:
    print('\n'.join(sorted(set(errors))))
    sys.exit(1)
print(f'PASS: local navigation and fragments across {len(pages)} built pages.')
