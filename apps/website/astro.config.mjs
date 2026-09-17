import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { readFileSync } from 'node:fs';
const product = JSON.parse(
  readFileSync(new URL('../../packages/product-info/product.json', import.meta.url), 'utf8'),
);
import { siteConfig } from './site-config.mjs';
const { site } = siteConfig;
export default defineConfig({
  ...(site ? { site } : {}),
  trailingSlash: 'always',
  integrations: [
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
      },
      sidebar: [
        {
          label: '开始使用',
          translations: { en: 'Get started' },
          items: [
            { label: '产品介绍', translations: { en: 'Overview' }, slug: 'docs/overview' },
            { label: '安装与下载', translations: { en: 'Installation' }, slug: 'docs/install' },
            { label: '三步连接', translations: { en: 'Quickstart' }, slug: 'docs/quickstart' },
          ],
        },
        {
          label: '管理连接',
          translations: { en: 'Connections' },
          items: [
            {
              label: '开启、关闭与恢复',
              translations: { en: 'Switching and recovery' },
              slug: 'docs/switch',
            },
            { label: '国内模型', translations: { en: 'Model providers' }, slug: 'docs/providers' },
            {
              label: '套餐与自定义接口',
              translations: { en: 'Plans and custom APIs' },
              slug: 'docs/relay',
            },
            {
              label: '配置字段与模型能力',
              translations: { en: 'Configuration and capabilities' },
              slug: 'docs/configuration',
            },
            { label: '通用设置', translations: { en: 'Settings' }, slug: 'docs/settings' },
            {
              label: '打开与重启 Codex',
              translations: { en: 'Launch and restart Codex' },
              slug: 'docs/launch',
            },
          ],
        },
        {
          label: '支持与说明',
          translations: { en: 'Help and reference' },
          items: [
            {
              label: 'Vercel 部署',
              translations: { en: 'Deploy to Vercel' },
              slug: 'docs/deployment',
            },
            {
              label: '兼容范围',
              translations: { en: 'Compatibility' },
              slug: 'docs/compatibility',
            },
            {
              label: '常见问题',
              translations: { en: 'Troubleshooting' },
              slug: 'docs/troubleshooting',
            },
            {
              label: '隐私与密钥',
              translations: { en: 'Privacy and API keys' },
              slug: 'docs/privacy',
            },
            {
              label: '开发与测试',
              translations: { en: 'Development and testing' },
              slug: 'docs/development',
            },
            {
              label: '开源致谢',
              translations: { en: 'Open-source credits' },
              slug: 'docs/open-source',
            },
            { label: '更新日志', translations: { en: 'Changelog' }, slug: 'docs/changelog' },
          ],
        },
      ],
    }),
  ],
});
