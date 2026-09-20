import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import react from '@astrojs/react';
import { readFileSync } from 'node:fs';
const product = JSON.parse(
  readFileSync(new URL('../../packages/product-info/product.json', import.meta.url), 'utf8'),
);
import { siteConfig } from './site-config.mjs';
const { site } = siteConfig;
export default defineConfig({
  ...(site ? { site } : {}),
  trailingSlash: 'always',
  redirects: {
    '/docs/overview/': { destination: '/', status: 308 },
    '/en/docs/overview/': { destination: '/en/', status: 308 },
    '/docs/relay/': { destination: '/docs/providers/', status: 308 },
    '/en/docs/relay/': { destination: '/en/docs/providers/', status: 308 },
    '/docs/launch/': { destination: '/docs/switch/', status: 308 },
    '/en/docs/launch/': { destination: '/en/docs/switch/', status: 308 },
  },
  integrations: [
    react(),
    starlight({
      title: 'Codex Switch',
      defaultLocale: 'root',
      locales: { root: { label: '简体中文', lang: 'zh-CN' }, en: { label: 'English', lang: 'en' } },
      description: 'Codex Switch 产品文档：连接国内模型与中转站，开启备份、关闭恢复。',
      logo: { src: './public/mark.png' },
      favicon: '/mark.png',
      social: [{ icon: 'github', label: 'GitHub', href: product.repository }],
      customCss: ['./src/styles/docs.css'],
      components: {
        Header: './src/components/docs/Header.astro',
        Head: './src/components/docs/Head.astro',
        LanguageSelect: './src/components/LanguageSwitch.astro',
        ThemeSelect: './src/components/docs/ThemeToggle.astro',
      },
      sidebar: [
        { label: '概览', translations: { en: 'Overview' }, slug: 'index' },
        {
          label: '开始使用',
          translations: { en: 'Get started' },
          items: [
            { label: '三步连接', translations: { en: 'Quickstart' }, slug: 'docs/quickstart' },
            { label: '安装与下载', translations: { en: 'Installation' }, slug: 'docs/install' },
          ],
        },
        {
          label: '功能说明',
          translations: { en: 'Features' },
          items: [
            {
              label: '开启与关闭',
              translations: { en: 'Enable and disable' },
              slug: 'docs/switch',
            },
            {
              label: '连接模型服务',
              translations: { en: 'Connect providers' },
              slug: 'docs/providers',
            },
            {
              label: '模型设置',
              translations: { en: 'Model settings' },
              slug: 'docs/configuration',
            },
            { label: '通用设置', translations: { en: 'Settings' }, slug: 'docs/settings' },
          ],
        },
        {
          label: '发布',
          translations: { en: 'Releases' },
          items: [
            { label: '更新日志', translations: { en: 'Changelog' }, slug: 'docs/changelog' },
            {
              label: '开源致谢',
              translations: { en: 'Open-source credits' },
              slug: 'docs/open-source',
            },
          ],
        },
      ],
    }),
  ],
});
