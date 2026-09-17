#!/usr/bin/env python3
"""Audit built bilingual pages and SEO output. No network or provider access."""
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit, unquote
import json
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1] / 'apps/website/dist'
REQUIRE_SITE = '--require-site' in sys.argv

class Page(HTMLParser):
    def __init__(self, file):
        super().__init__()
        self.lang = ''
        self.titles = []
        self.meta = {}
        self.links = []
        self.anchors = []
        self.h1 = 0
        self.schemas = []
        self.capture = None
        self.buffer = ''
        self.feed(file.read_text())
    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == 'html': self.lang = a.get('lang', '')
        if tag == 'meta': self.meta.setdefault(a.get('name', a.get('property', '')), []).append(a.get('content', ''))
        if tag == 'link': self.links.append(a)
        if tag == 'a': self.anchors.append(a)
        if tag == 'h1': self.h1 += 1
        if tag == 'title' or (tag == 'script' and a.get('type') == 'application/ld+json'):
            self.capture = tag
            self.buffer = ''
    def handle_data(self, data):
        if self.capture: self.buffer += data
    def handle_endtag(self, tag):
        if tag != self.capture: return
        if tag == 'title': self.titles.append(self.buffer.strip())
        else: self.schemas.append(json.loads(self.buffer))
        self.capture = None

files = list(ROOT.rglob('*.html'))
if not files: sys.exit('Build the website before checking SEO.')
pages = {}
for file in files:
    name = file.relative_to(ROOT).as_posix()
    path = '/' + (name[:-10] if name.endswith('index.html') else name)
    pages[path] = Page(file)
errors = []
def check(ok, message):
    if not ok: errors.append(message)

def is404(path):
    return path in ('/404.html', '/404/', '/en/404.html', '/en/404/')

def localized(base, locale):
    return '/en' + base if locale == 'en' else base

def basepath(path):
    return path[3:] if path.startswith('/en/') else path

canonicals = set()
titles, descriptions = [], []
site_origin = None
for path, page in pages.items():
    locale = 'en' if path.startswith('/en/') else 'zh-CN'
    check(page.lang.lower() == locale.lower(), f'{path}: incorrect html language')
    check(len(page.titles) == 1 and bool(page.titles[0]), f'{path}: needs exactly one title')
    check(page.h1 == 1, f'{path}: needs exactly one H1')
    for key in ('description', 'robots', 'og:title', 'og:description', 'og:locale', 'twitter:title', 'twitter:description', 'twitter:card'):
        check(len(page.meta.get(key, [])) == 1 and bool(page.meta[key][0]), f'{path}: missing or duplicate {key}')
    description = page.meta.get('description', [''])[0]
    check(page.meta.get('og:description') == [description], f'{path}: inconsistent share description')
    check(page.meta.get('og:title') == page.titles, f'{path}: inconsistent share title')
    canonical = [a.get('href') for a in page.links if a.get('rel') == 'canonical']
    alternates = [a for a in page.links if a.get('rel') == 'alternate']
    robots = page.meta.get('robots', [''])[0]
    if not is404(path):
        if '--expect-noindex' in sys.argv: check('noindex' in robots, f'{path}: expected preview noindex')
        if '--expect-indexable' in sys.argv: check('noindex' not in robots and 'index' in robots, f'{path}: expected production indexing')
    if is404(path):
        check('noindex' in robots, f'{path}: 404 must be noindex')
        check(not alternates, f'{path}: 404 must not advertise language alternates')
        check(not canonical and not page.schemas, f'{path}: 404 must not advertise canonical or structured data')
        continue
    titles += page.titles
    descriptions.append(description)
    base = basepath(path)
    counterpart = localized(base, 'zh-CN' if locale == 'en' else 'en')
    check(counterpart in pages, f'{path}: missing translation {counterpart}')
    switches = [a for a in page.anchors if 'language-switch' in a.get('class', '').split()]
    check(any(a.get('href') == counterpart for a in switches), f'{path}: switch must link to equivalent page')
    # All website navigation should stay in the current language, except the explicit switch.
    for a in page.anchors:
        href = a.get('href', '')
        if 'language-switch' in a.get('class', '').split(): continue
        if locale == 'en' and (href.startswith('/docs/') or href == '/' or href == '/download/'):
            errors.append(f'{path}: English navigation points to Chinese page {href}')
    check(len(page.schemas) == 2, f'{path}: missing page/product/breadcrumb structured data')
    for schema in page.schemas:
        check(schema.get('@context') == 'https://schema.org', f'{path}: invalid JSON-LD context')
    if canonical:
        check(len(canonical) == 1 and bool(canonical[0]), f'{path}: duplicate or empty canonical')
        url = urlsplit(canonical[0])
        check(url.scheme == 'https' and bool(url.netloc) and url.path == path, f'{path}: canonical must be an absolute self URL')
        origin = f'{url.scheme}://{url.netloc}'
        if site_origin is None: site_origin = origin
        check(site_origin == origin, f'{path}: mixed canonical origins')
        canonicals.add(canonical[0])
        expected = {'zh-CN': origin + base, 'en': origin + '/en' + base, 'x-default': origin + base}
        check(len(alternates) == 3 and {a.get('hreflang'): a.get('href') for a in alternates} == expected, f'{path}: incorrect hreflang cluster')
        check(page.meta.get('og:url') == canonical, f'{path}: inconsistent og:url')
        for key in ('og:image', 'twitter:image'):
            values = page.meta.get(key, [])
            check(len(values) == 1 and values[0].startswith(origin + '/'), f'{path}: invalid {key}')
            if values: check((ROOT / unquote(urlsplit(values[0]).path).lstrip('/')).is_file(), f'{path}: missing share image')
        check(page.schemas[0].get('url') == canonical[0] and page.schemas[0].get('inLanguage') == locale, f'{path}: inconsistent page schema')
    else:
        check(not REQUIRE_SITE, f'{path}: PUBLIC_SITE_URL is required')
        check('noindex' in robots, f'{path}: unconfigured site must be noindex')
        check(not alternates, f'{path}: unconfigured site must not invent alternate URLs')

for label, values in [('title', titles), ('description', descriptions)]:
    for value, count in Counter(values).items():
        check(count == 1, f'duplicate {label}: {value}')
robots_file = ROOT / 'robots.txt'
check(robots_file.exists(), 'missing robots.txt')
robots = robots_file.read_text() if robots_file.exists() else ''
if site_origin:
    sitemap_urls = set()
    for file in ROOT.glob('sitemap-*.xml'):
        tree = ET.parse(file)
        for node in tree.findall('{*}url'):
            loc = node.find('{*}loc')
            if loc is not None:
                sitemap_urls.add(loc.text)
                base = basepath(urlsplit(loc.text).path)
                expected = {'zh-CN': site_origin + base, 'en': site_origin + '/en' + base}
                actual = {a.get('hreflang'): a.get('href') for a in node.findall('{http://www.w3.org/1999/xhtml}link')}
                check(actual == expected, f'{loc.text}: incorrect sitemap language alternates')
    check(sitemap_urls == canonicals, f'sitemap coverage mismatch: missing={sorted(canonicals - sitemap_urls)}, extra={sorted(sitemap_urls - canonicals)}')
    check((ROOT / 'sitemap-index.xml').exists(), 'missing sitemap index')
    indexable = all('noindex' not in p.meta.get('robots', [''])[0] for path,p in pages.items() if not is404(path))
    check(('Allow: /' in robots and f'Sitemap: {site_origin}/sitemap-index.xml' in robots) if indexable else 'Disallow: /' in robots, 'robots and page indexing policy disagree')
else:
    check('Disallow: /' in robots and 'Sitemap:' not in robots, 'unconfigured robots must disallow crawling')

if errors:
    print('\n'.join('FAIL: ' + e for e in errors))
    sys.exit(1)
print(f'PASS: {len(pages)} pages; paired languages, navigation, metadata, structured data, robots and sitemap policy.')
