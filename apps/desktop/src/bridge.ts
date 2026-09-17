import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import registry from '../../../packages/provider-registry/providers.json';
export type Protocol = 'chat' | 'responses';
export interface ModelOptions {
  endpoint?: string;
  protocol?: Protocol;
  fullUrl?: boolean;
  baseInstructions?: string;
  contextWindow?: number;
  reasoningLevels?: string[];
  defaultReasoningLevel?: string;
  inputModalities?: string[];
  parallelToolCalls?: boolean;
}
export interface ConnectionOptions {
  fullUrl?: boolean;
  endpointCandidates?: string[];
  modelOverrides?: Partial<Record<string, ModelOptions>>;
  chatReasoning?: {
    supportsThinking?: boolean;
    supportsEffort?: boolean;
    thinkingParam?: string;
    effortParam?: string;
    effortValueMode?: string;
    outputFormat?: string;
  } | null;
}
export interface RoutingSettings {
  remoteCompaction: boolean;
}
export interface Profile {
  id: string;
  name: string;
  presetId: string;
  endpoint: string;
  models: string[];
  protocol: Protocol;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface EndpointVariant {
  name: string;
  endpoint: string;
  model: string;
  protocol: Protocol;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface Preset {
  variants?: EndpointVariant[];
  id: string;
  name: string;
  endpoint: string;
  model: string;
  protocol: Protocol;
  keyUrl: string;
  docsUrl: string;
  envKey: string;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface Snapshot {
  profiles: Profile[];
  presets: Preset[];
  selectedProfiles: string[];
  needsApply: boolean;
  enabled: boolean;
  routing: boolean;
  pendingReload: boolean;
  activeRequests: number;
  app: { installed: boolean; running: boolean };
  configPath: string;
  autostart: boolean;
  routingSettings?: RoutingSettings;
  unavailable?: { id: string; message: string }[];
  trayFeedback?: { message: string; isError: boolean } | null;
}
export interface Receipt {
  ok: boolean;
  status: number | null;
  message: string;
  elapsedMs: number;
}
export const isPreview = !(window as unknown as { __TAURI_INTERNALS__?: unknown })
  .__TAURI_INTERNALS__;
export async function subscribeToTray(onChange: () => void): Promise<() => void> {
  if (isPreview) return () => {};
  return listen('tray-state-changed', onChange);
}
const state: Snapshot = {
  profiles: [],
  presets: registry as Preset[],
  selectedProfiles: [],
  needsApply: false,
  enabled: false,
  routing: false,
  pendingReload: false,
  activeRequests: 0,
  app: { installed: true, running: false },
  configPath: '~/.codex/config.toml',
  autostart: false,
};
export async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isPreview) return invoke<T>(command, args);
  // UI preview has no native, network, file, or credential side effects.
  switch (command) {
    case 'snapshot':
      return structuredClone(state) as T;
    case 'save_profile': {
      if (state.enabled) throw new Error('Codex Switch 已开启，请先关闭后再新增、编辑或删除配置。');
      const p = structuredClone(args.profile as Profile);
      const original = state.profiles.find((other) => other.id === p.id);
      if (original && original.presetId !== p.presetId)
        throw new Error('已有配置不能更换厂商，请新增配置。');
      p.name = p.name.trim();
      if (
        state.profiles.some(
          (other) => other.id !== p.id && other.name.toLowerCase() === p.name.toLowerCase(),
        )
      )
        throw new Error('配置名称已存在，请换一个名称。');
      p.id ||= crypto.randomUUID();
      const index = state.profiles.findIndex((other) => other.id === p.id);
      if (index < 0) state.profiles.push(p);
      else state.profiles[index] = p;
      state.needsApply = true;
      return p as T;
    }
    case 'save_routing_settings':
      if (state.enabled) throw new Error('请先关闭 Codex Switch。');
      state.routingSettings = structuredClone(args.settings as RoutingSettings);
      break;
    case 'probe_endpoint':
      return { ok: true, status: 200, message: '界面预览：未发送请求', elapsedMs: 0 } as T;
    case 'cancel_validation':
      break;
    case 'check_connection':
      return { ok: true, status: 200, message: '示例连接验证通过', elapsedMs: 0 } as T;
    case 'set_enabled':
      if (args.enabled && !state.selectedProfiles.length) throw new Error('请至少勾选一个配置。');
      state.enabled = Boolean(args.enabled);
      state.routing = state.enabled;
      state.needsApply = false;
      state.pendingReload = state.app.running;
      break;
    case 'select_profiles':
      if (state.enabled && !(args.ids as string[]).length)
        throw new Error('开启期间必须保留至少一个配置；如需停用，请关闭 Codex Switch。');
      state.selectedProfiles = [...(args.ids as string[])];
      state.needsApply = !state.enabled;
      if (state.enabled) {
        state.routing = true;
        state.pendingReload = state.app.running;
      }
      break;
    case 'delete_profile':
      if (state.enabled) throw new Error('Codex Switch 已开启，请先关闭后再新增、编辑或删除配置。');
      state.profiles = state.profiles.filter((p) => p.id !== args.id);
      state.selectedProfiles = state.selectedProfiles.filter((id) => id !== args.id);
      state.needsApply = true;
      break;
    case 'recover_connection':
      if (!state.enabled || !state.selectedProfiles.length)
        throw new Error('请至少选择一个配置并开启 Codex Switch。');
      state.routing = state.enabled;
      state.pendingReload = state.app.running;
      break;
    case 'open_codex':
      state.app.running = true;
      state.pendingReload = false;
      break;
    case 'restart_codex':
      state.app.running = true;
      state.pendingReload = false;
      state.routing = state.enabled;
      break;
    case 'set_autostart':
      state.autostart = Boolean(args.enabled);
      break;
    case 'open_link':
      window.open(String(args.url), '_blank', 'noopener,noreferrer');
      break;
    case 'quit':
      break;
    default:
      throw new Error('Unknown preview command');
  }
  return undefined as T;
}
