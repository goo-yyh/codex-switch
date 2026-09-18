import { describe, expect, it } from 'vitest';
import registry from '../../../../../packages/provider-registry/providers.json';
import models from '../../../../../packages/provider-registry/models.json';
import type { Preset, Profile } from '../../api/bridge';
import { selectModels, withReasoningDefaults } from './modelDefaults';

const presets = registry as Preset[];
const profile = (preset: Preset): Profile => ({
  id: 'saved',
  name: 'Saved',
  presetId: preset.id,
  endpoint: preset.endpoint,
  models: [preset.model],
  protocol: preset.protocol,
  contextWindow: 32768,
});

describe('official reasoning defaults', () => {
  it('covers all built-in models and keeps every default within the supported list', () => {
    for (const preset of presets) {
      for (const model of models[preset.id as keyof typeof models]) {
        const spec = withReasoningDefaults(profile(preset), model, preset);
        expect(spec.reasoningLevels?.length, model).toBeGreaterThan(0);
        if (spec.defaultReasoningLevel) {
          expect(spec.reasoningLevels, model).toContain(spec.defaultReasoningLevel);
        } else {
          expect(spec.reasoningNote).toContain('未单列');
        }
      }
      for (const variant of preset.variants ?? []) {
        for (const spec of Object.values(variant.options?.modelOverrides ?? {})) {
          if (spec?.defaultReasoningLevel)
            expect(spec.reasoningLevels).toContain(spec.defaultReasoningLevel);
        }
      }
    }
  });
  it.each([
    ['zhipu', 'glm-5.3-flash', 'max', ['low', 'high', 'max']],
    ['zhipu', 'glm-5.2', 'max', ['none', 'high', 'max']],
    ['qianwen', 'qwen3.8-27b', 'xhigh', ['none', 'low', 'medium', 'xhigh']],
    ['qianwen', 'qwen3.7-plus', 'medium', ['none', 'low', 'medium', 'high', 'xhigh']],
    ['minimax', 'MiniMax-M3', 'none', ['none', 'high']],
    ['minimax', 'MiniMax-M2.7-highspeed', 'high', ['high']],
    ['kimi', 'kimi-k3', 'max', ['low', 'high', 'max']],
    ['deepseek', 'deepseek-flash', 'high', ['none', 'low', 'high', 'max']],
  ])('loads %s / %s defaults into old profiles', (provider, model, value, levels) => {
    const preset = presets.find((p) => p.id === provider)!;
    const saved = profile(preset);
    const spec = withReasoningDefaults(saved, model as string, preset);
    expect(spec.defaultReasoningLevel).toBe(value);
    expect(spec.reasoningLevels).toEqual(levels);
    expect(saved.options).toBeUndefined();
  });
  it('preserves explicit overrides and does not copy unrelated preset fields', () => {
    const preset = presets.find((p) => p.id === 'zhipu')!;
    const saved = profile(preset);
    saved.options = {
      modelOverrides: {
        'glm-5.3-flash': {
          reasoningLevels: ['low', 'high'],
          defaultReasoningLevel: 'low',
          endpoint: 'https://custom.example/v1',
          contextWindow: 65536,
        },
      },
    };
    const spec = withReasoningDefaults(saved, 'glm-5.3-flash', preset);
    expect(spec).toEqual(saved.options.modelOverrides?.['glm-5.3-flash']);
    delete saved.options.modelOverrides!['glm-5.3-flash']!.defaultReasoningLevel;
    expect(withReasoningDefaults(saved, 'glm-5.3-flash', preset).defaultReasoningLevel).toBe(
      'high',
    );
    expect(withReasoningDefaults(saved, 'private-model', preset)).toEqual({});
  });
  it('uses the official MiniMax Codex plan default independently of the ordinary API', () => {
    const preset = presets.find((p) => p.id === 'minimax')!;
    const saved = profile(preset);
    saved.endpoint = 'https://api.minimax.cn/v1';
    const spec = withReasoningDefaults(saved, 'MiniMax-M3', preset);
    expect(spec.defaultReasoningLevel).toBe('high');
    expect(spec.reasoningNote).toContain('Codex');
  });
  it('keeps unspecified official defaults automatic even in a new preset-based profile', () => {
    const preset = presets.find((p) => p.id === 'qianwen')!;
    const saved = { ...profile(preset), options: structuredClone(preset.options) };
    expect(
      withReasoningDefaults(saved, 'qwen3.7-flash', preset).defaultReasoningLevel,
    ).toBeUndefined();
    expect(
      withReasoningDefaults(saved, 'qwen3.6-max-preview', preset).defaultReasoningLevel,
    ).toBeUndefined();
  });
});

describe('remote model selection', () => {
  it('copies new model defaults without scaling context or changing selected/local model settings', () => {
    const preset = structuredClone(presets[0]);
    const next = {
      contextWindow: 131072,
      reasoningLevels: ['high'],
      defaultReasoningLevel: 'high',
    };
    preset.options!.modelOverrides!['test-new'] = next;
    preset.variants![0].options!.modelOverrides!['test-new'] = next;
    const saved = profile(preset);
    saved.options = {
      modelOverrides: { [preset.model]: { contextWindow: 65536 }, 'test-custom': {} },
    };
    const selected = selectModels(saved, [...saved.models, 'test-new', 'test-custom'], preset);
    expect(selected.models[0]).toBe(saved.models[0]);
    expect(selected.options!.modelOverrides!['test-new']).toEqual(next);
    expect(selected.options!.modelOverrides![preset.model]).toEqual({ contextWindow: 65536 });
    expect(selected.options!.modelOverrides!['test-custom']).toEqual({});
    expect(saved.options.modelOverrides!['test-new']).toBeUndefined();
    next.contextWindow = 4096;
    expect(selected.options!.modelOverrides!['test-new']!.contextWindow).toBe(131072);
  });
  it('does not overwrite settings for a manually added model or apply defaults to another endpoint', () => {
    const preset = structuredClone(presets[0]);
    preset.options!.modelOverrides!['test-new'] = { contextWindow: 131072 };
    const saved = profile(preset);
    saved.models.push('test-new');
    expect(
      selectModels(saved, saved.models, preset).options?.modelOverrides?.['test-new'],
    ).toBeUndefined();
    saved.models = [preset.model];
    saved.endpoint = 'https://custom.example/v1';
    expect(
      selectModels(saved, [...saved.models, 'test-new'], preset).options?.modelOverrides?.[
        'test-new'
      ],
    ).toBeUndefined();
  });
});
