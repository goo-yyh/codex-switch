import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
export type * from './types';
export const isPreview = !(window as unknown as { __TAURI_INTERNALS__?: unknown })
  .__TAURI_INTERNALS__;
export async function subscribeToAppChanges(onChange: () => void): Promise<() => void> {
  if (isPreview) return () => {};
  const stopTray = await listen('tray-state-changed', onChange);
  try {
    const stopRegistry = await listen('model-registry-updated', onChange);
    return () => {
      stopTray();
      stopRegistry();
    };
  } catch (error) {
    stopTray();
    throw error;
  }
}

export async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isPreview) return invoke<T>(command, args);
  const { previewCall } = await import('./preview');
  return previewCall<T>(command, args);
}
