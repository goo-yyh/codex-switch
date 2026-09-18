import { afterEach, describe, expect, it, vi } from 'vitest';
import { displayText, translate } from './index';
import { localizeMessage } from './messages';
import { en } from './en';
import registry from '../../../../packages/provider-registry/providers.json';
import { documentationUrl } from '../product';

afterEach(() => {
  vi.unstubAllEnvs();
  window.localStorage.clear();
});

describe('application localization', () => {
  it('translates model display metadata without changing arbitrary user values', () => {
    expect(displayText('en', '智谱 GLM')).toBe('Zhipu GLM');
    expect(displayText('en', '我的模型 high')).toBe('我的模型 high');
    expect(translate('en', '编辑 {value0}', { value0: '智谱私人配置' })).toBe('Edit 智谱私人配置');
    expect(translate('zh-CN', '已选 {count}', { count: 2 })).toBe('已选 2');
  });
  it('covers preset names and notes while preserving interpolation placeholders', () => {
    function visit(value: unknown) {
      if (!value || typeof value !== 'object') return;
      for (const [key, item] of Object.entries(value)) {
        if (
          ['name', 'reasoningNote', 'contextNote'].includes(key) &&
          typeof item === 'string' &&
          /\p{Script=Han}/u.test(item)
        ) {
          expect(Object.hasOwn(en, item), item).toBe(true);
        }
        visit(item);
      }
    }
    visit(registry);
    for (const [key, value] of Object.entries(en)) {
      expect(value.match(/\{\w+\}/g)?.sort()).toEqual(key.match(/\{\w+\}/g)?.sort());
      expect(value).not.toMatch(/\p{Script=Han}/u);
    }
  });
  it('translates diagnostics but preserves user names and unknown upstream details', () => {
    expect(localizeMessage('en', 'Error: 请填写 API Key。')).toBe('Error: Enter an API key.');
    expect(localizeMessage('en', 'glm-5.3：密钥无效，或当前套餐没有此权限。')).toBe(
      'glm-5.3: Invalid key or insufficient permissions for this plan.',
    );
    expect(localizeMessage('en', '工作账号 的密钥为空。')).toBe('The key for 工作账号 is empty.');
    expect(localizeMessage('en', '配置测试通过，2 个模型请求成功。 · 20 ms')).toBe(
      'Configuration test passed: 2 model requests succeeded. · 20 ms',
    );
    expect(localizeMessage('en', 'Upstream error 529: trace=abc')).toBe(
      'Upstream error 529: trace=abc',
    );
    expect(localizeMessage('en', '上游详情：trace=abc')).toBe('上游详情：trace=abc');
    expect(localizeMessage('en', 'constructor')).toBe('constructor');
  });
  it('persists preview language across module reloads', async () => {
    const first = await import('../api/preview');
    await first.previewCall('set_locale', { locale: 'en' });
    vi.resetModules();
    const reloaded = await import('../api/preview');
    expect(await reloaded.previewCall('snapshot')).toMatchObject({ locale: 'en', enabled: false });
    await reloaded.previewCall('set_locale', { locale: 'zh-CN' });
    expect(window.localStorage.getItem('codex-switch-locale')).toBe('zh-CN');
  });
  it('opens equivalent docs routes and retains HTTPS validation', () => {
    vi.stubEnv('VITE_PUBLIC_DOCS_URL', 'https://docs.example.test/docs/quickstart/');
    expect(documentationUrl('en')).toBe('https://docs.example.test/en/docs/quickstart/');
    vi.stubEnv('VITE_PUBLIC_DOCS_URL', 'https://docs.example.test/en/docs/settings/');
    expect(documentationUrl('zh-CN')).toBe('https://docs.example.test/docs/settings/');
    vi.stubEnv('VITE_PUBLIC_DOCS_URL', 'http://invalid.test/');
    expect(documentationUrl('en')).toContain('/content/docs/en/docs/quickstart.md');
  });
});
