import registry from '../../../../packages/provider-registry/providers.json';
import type { Snapshot, Preset, Profile, RoutingSettings } from './types';

const languageKey = 'codex-switch-locale';
function storedLocale(): 'zh-CN' | 'en' {
  try {
    return window.localStorage.getItem(languageKey) === 'en' ? 'en' : 'zh-CN';
  } catch {
    return 'zh-CN';
  }
}
const state: Snapshot = {
  locale: storedLocale(),
  profiles: [],
  presets: registry as Preset[],
  selectedProfiles: [],
  enabled: false,
  routing: false,
  pendingReload: false,
  activeRequests: 0,
  app: { installed: true, running: false },
  configPath: '~/.codex/config.toml',
  autostart: false,
};
export async function previewCall<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  // UI preview has no native, network, file, or credential side effects.
  switch (command) {
    case 'check_update':
      // Explicit visual-test fixture; previews never contact GitHub or install updates.
      return (
        new URLSearchParams(window.location.search).get('previewUpdate') === 'available'
          ? {
              version: '0.2.0',
              url: 'https://github.com/goo-yyh/codex-switch/releases/download/v0.2.0/Codex_0.2.0_aarch64.dmg',
            }
          : null
      ) as T;
    case 'snapshot':
      return structuredClone(state) as T;
    case 'save_profile': {
      if (state.enabled) throw new Error('Codex Switch 已开启，请先关闭服务，再修改配置或设置。');
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
      return p as T;
    }
    case 'set_locale': {
      if (state.enabled) throw new Error('请先关闭服务，再修改配置或设置。');
      if (args.locale !== 'en' && args.locale !== 'zh-CN') throw new Error('Invalid language');
      // Persistence failure must not report success or change the displayed language.
      window.localStorage.setItem(languageKey, args.locale);
      state.locale = args.locale;
      break;
    }
    case 'save_routing_settings':
      if (state.enabled) throw new Error('请先关闭 Codex Switch。');
      state.routingSettings = structuredClone(args.settings as RoutingSettings);
      break;
    case 'probe_endpoint':
      return { ok: true, status: 200, message: '界面预览：未发送请求', elapsedMs: 0 } as T;
    case 'cancel_validation':
      break;
    case 'set_enabled':
      if (args.enabled && !state.selectedProfiles.length) throw new Error('请至少勾选一个配置。');
      state.enabled = Boolean(args.enabled);
      state.routing = state.enabled;
      state.pendingReload = state.app.running;
      break;
    case 'select_profiles':
      if (state.enabled) throw new Error('请先关闭服务，再修改配置或设置。');
      state.selectedProfiles = [...(args.ids as string[])];
      break;
    case 'delete_profile':
      if (state.enabled) throw new Error('Codex Switch 已开启，请先关闭服务，再修改配置或设置。');
      state.profiles = state.profiles.filter((p) => p.id !== args.id);
      state.selectedProfiles = state.selectedProfiles.filter((id) => id !== args.id);
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
      if (state.enabled) throw new Error('请先关闭服务，再修改配置或设置。');
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
