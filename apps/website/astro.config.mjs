import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { readFileSync } from 'node:fs';
const product = JSON.parse(
  readFileSync(new URL('../../packages/product-info/product.json', import.meta.url), 'utf8'),
);
export default defineConfig({
  ...(process.env.PUBLIC_SITE_URL ? { site: process.env.PUBLIC_SITE_URL } : {}),
  integrations: [
    starlight({
      title: 'Codex Switch',
      defaultLocale: 'root',
      locales: { root: { label: '简体中文', lang: 'zh-CN' } },
      description: 'Codex Switch 产品文档：连接国内模型与中转站，开启备份、关闭恢复。',
      logo: { src: './public/mark.png' },
      favicon: '/mark.png',
      social: [{ icon: 'github', label: 'GitHub', href: product.repository }],
      customCss: ['./src/styles/docs.css'],
      components: { Header: './src/components/docs/Header.astro' },
      sidebar: [
        {
          label: '开始使用',
          items: [
            { label: '产品介绍', slug: 'docs/overview' },
            { label: '安装与下载', slug: 'docs/install' },
            { label: '三步连接', slug: 'docs/quickstart' },
          ],
        },
        {
          label: '管理连接',
          items: [
            { label: '开启、关闭与恢复', slug: 'docs/switch' },
            { label: '国内模型', slug: 'docs/providers' },
            { label: '套餐与自定义接口', slug: 'docs/relay' },
            { label: '配置字段与模型能力', slug: 'docs/configuration' },
            { label: '通用设置', slug: 'docs/settings' },
            { label: '打开与重启 Codex', slug: 'docs/launch' },
          ],
        },
        {
          label: '支持与说明',
          items: [
            { label: '兼容范围', slug: 'docs/compatibility' },
            { label: '常见问题', slug: 'docs/troubleshooting' },
            { label: '隐私与密钥', slug: 'docs/privacy' },
            { label: '开发与测试', slug: 'docs/development' },
            { label: '开源致谢', slug: 'docs/open-source' },
            { label: '更新日志', slug: 'docs/changelog' },
          ],
        },
      ],
    }),
  ],
});
