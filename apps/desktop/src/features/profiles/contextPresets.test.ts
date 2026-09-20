import { describe, expect, it } from 'vitest';
import registry from '../../../../../packages/provider-registry/providers.json';
import candidates from '../../../../../packages/provider-registry/models.json';
import type { Preset } from '../../api/bridge';

const presets = registry as Preset[];
// Official capacity bases and the two conservative choices are documented in
// docs/testing/model-context-windows-2026-09-17.md. These are upstream capacities,
// not already-discounted values, so repeated 80% scaling fails this check.
const capacities: Record<string, Record<string, number>> = {
  zhipu: {
    'glm-5.3': 1_000_000,
    'glm-5.3-flash': 1_000_000,
    'glm-5.3-flashx': 1_000_000,
    'glm-5.2': 1_000_000,
    'glm-5.1': 204_800,
    'glm-5': 204_800,
    'glm-5-turbo': 204_800,
    'glm-5v-turbo': 204_800,
    'glm-4.7': 204_800,
    'glm-4.7-flashx': 204_800,
    'glm-4.7-flash': 204_800,
  },
  deepseek: {
    'deepseek-flash': 1_048_576,
    'deepseek-v4-flash': 1_048_576,
    'deepseek-v4-flash-vision-exp': 1_048_576,
    'deepseek-v4-pro': 1_048_576,
  },
  kimi: {
    'kimi-k3': 1_048_576,
    'kimi-k2.7-code': 262_144,
    'kimi-k2.7-code-highspeed': 262_144,
    'kimi-k2.6': 262_144,
  },
  qianwen: {
    'qwen3.8-max': 1_000_000,
    'qwen3.8-flash': 1_000_000,
    'qwen3.8-omni-flash': 1_000_000,
    'qwen3.8-2.4t-a95b': 1_000_000,
    'qwen3.8-27b': 1_000_000,
    'qwen3.7-max': 1_000_000,
    'qwen3.7-plus': 1_000_000,
    'qwen3.7-flash': 1_000_000,
    'qwen3.6-max-preview': 262_144,
    'qwen3.6-plus': 1_000_000,
    'qwen3.6-flash': 1_000_000,
  },
  minimax: {
    'MiniMax-M3': 1_000_000,
    'MiniMax-M2.7': 204_800,
    'MiniMax-M2.7-highspeed': 204_800,
    'MiniMax-M2.5': 204_800,
    'MiniMax-M2.5-highspeed': 204_800,
    'MiniMax-M2.1': 204_800,
    'MiniMax-M2.1-highspeed': 204_800,
    'MiniMax-M2': 204_800,
  },
};

describe('context capacity presets', () => {
  it('gives every built-in candidate its own verified 80% context instead of inheriting', () => {
    for (const preset of presets) {
      const models = candidates[preset.id as keyof typeof candidates];
      // Removed suggestions retain capability metadata for saved/custom models.
      expect(Object.keys(capacities[preset.id])).toEqual(expect.arrayContaining(models));
      for (const model of models) {
        expect(preset.options?.modelOverrides?.[model]?.contextWindow, model).toBe(
          Math.floor((capacities[preset.id][model] * 4) / 5),
        );
      }
      expect(preset.contextWindow).toBe(
        preset.options?.modelOverrides?.[preset.model]?.contextWindow,
      );
    }
  });

  it('covers every variant and respects Kimi membership limits and upgraded aliases', () => {
    const kimiPlan = {
      'kimi-for-coding': 1_048_576,
      'kimi-for-coding-highspeed': 262_144,
      k3: 262_144,
      'k3-256k': 262_144,
    };
    for (const preset of presets) {
      for (const variant of preset.variants ?? []) {
        const capacity: Record<string, number> =
          preset.id === 'kimi' && variant.endpoint.includes('/coding')
            ? kimiPlan
            : capacities[preset.id];
        for (const [model, spec] of Object.entries(variant.options?.modelOverrides ?? {})) {
          expect(capacity[model], model).toBeDefined();
          expect(spec?.contextWindow, model).toBe(Math.floor((capacity[model] * 4) / 5));
        }
        expect(variant.contextWindow).toBe(
          variant.options?.modelOverrides?.[variant.model]?.contextWindow,
        );
      }
    }
    const plan = presets.find((p) => p.id === 'kimi')!.variants![1];
    expect(plan.options?.modelOverrides?.k3?.contextNote).toContain('Allegretto');
    expect(plan.options?.modelOverrides?.k3?.defaultReasoningLevel).toBe('high');
    expect(plan.options?.modelOverrides?.['kimi-for-coding']?.defaultReasoningLevel).toBe('max');
  });
});
