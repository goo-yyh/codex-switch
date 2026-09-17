import type { ModelOptions, Preset, Profile } from '../../api/bridge';

// Fill missing reasoning fields only. Never replace saved connection settings or
// explicit model customizations when the bundled presets change.
export function withReasoningDefaults(
  profile: Profile,
  model: string,
  preset?: Preset,
): ModelOptions {
  const saved = profile.options?.modelOverrides?.[model] ?? {};
  const variant = preset?.variants?.find(
    (v) => v.endpoint === profile.endpoint && v.protocol === profile.protocol,
  );
  const defaults =
    variant?.options?.modelOverrides?.[model] ?? preset?.options?.modelOverrides?.[model];
  const levels = saved.reasoningLevels ?? defaults?.reasoningLevels;
  const preferred = saved.defaultReasoningLevel ?? defaults?.defaultReasoningLevel;
  const defaultLevel =
    saved.defaultReasoningLevel ??
    (preferred
      ? levels?.includes(preferred)
        ? preferred
        : saved.reasoningLevels?.at(-1)
      : undefined);
  return {
    ...saved,
    ...(levels ? { reasoningLevels: [...levels] } : {}),
    ...(defaultLevel ? { defaultReasoningLevel: defaultLevel } : {}),
    ...(defaults?.reasoningNote ? { reasoningNote: defaults.reasoningNote } : {}),
  };
}
