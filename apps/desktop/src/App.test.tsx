import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, act, within } from '@testing-library/react';
import App from './App';
import { call, subscribeToTray, type Snapshot, type Profile, type Preset } from './bridge';
import registry from '../../../packages/provider-registry/providers.json';
vi.mock('./bridge', () => ({
  isPreview: true,
  call: vi.fn(),
  subscribeToTray: vi.fn(async () => () => {}),
}));
const profile = (id = 'one', name = 'Kimi'): Profile => ({
  id,
  name,
  presetId: 'kimi',
  endpoint: 'https://api.moonshot.cn/v1',
  models: ['kimi-k3', 'kimi-k2.7-code'],
  protocol: 'chat',
  contextWindow: 32768,
});
function setup(profiles: Profile[] = [profile()]) {
  const state: Snapshot = {
    profiles,
    presets: registry as Preset[],
    selectedProfiles: [],
    needsApply: true,
    enabled: false,
    routing: false,
    pendingReload: false,
    activeRequests: 0,
    app: { installed: true, running: false },
    configPath: '/temporary/config.toml',
    autostart: false,
  };
  vi.mocked(call).mockImplementation(async (command, args) => {
    if (command === 'snapshot') return structuredClone(state) as never;
    if (command === 'save_profile') {
      const p = { ...(args?.profile as Profile), id: (args?.profile as Profile).id || 'saved' };
      const index = state.profiles.findIndex((v) => v.id === p.id);
      if (index < 0) state.profiles.push(p);
      else state.profiles[index] = p;
      return p as never;
    }
    if (command === 'probe_endpoint')
      return { ok: true, status: 200, message: '配置测试通过', elapsedMs: 10 } as never;
    if (command === 'select_profiles') {
      const ids = args?.ids as string[];
      if (state.enabled && !ids.length) throw new Error('开启期间必须保留至少一个配置。');
      state.selectedProfiles = ids;
      state.needsApply = !state.enabled;
    }
    if (command === 'set_enabled') {
      state.enabled = Boolean(args?.enabled);
      state.routing = state.enabled;
      state.needsApply = false;
      state.pendingReload = state.app.running;
    }
    if (command === 'delete_profile') {
      state.profiles = state.profiles.filter((p) => p.id !== args?.id);
      state.selectedProfiles = state.selectedProfiles.filter((id) => id !== args?.id);
    }
    if (command === 'save_routing_settings') {
      state.routingSettings = structuredClone(
        args?.settings as NonNullable<Snapshot['routingSettings']>,
      );
    }
    return undefined as never;
  });
  return state;
}
async function addKimi() {
  fireEvent.click((await screen.findAllByRole('button', { name: '新增配置' }))[0]);
  fireEvent.click(screen.getByRole('button', { name: 'Kimi' }));
}
describe('configuration lifecycle', () => {
  beforeEach(() => vi.clearAllMocks());
  it('refreshes selection and feedback after tray actions without applying again', async () => {
    const state = setup([profile(), profile('two', '工作账号')]);
    let notify = () => {};
    const stop = vi.fn();
    vi.mocked(subscribeToTray).mockImplementationOnce(async (cb) => {
      notify = cb;
      return stop;
    });
    const view = render(<App />);
    await screen.findByRole('heading', { name: '我的配置' });
    state.selectedProfiles = ['two'];
    state.enabled = state.routing = true;
    state.needsApply = false;
    state.trayFeedback = { message: '已从托盘切换到工作账号。', isError: false };
    act(() => notify());
    await screen.findByText('已从托盘切换到工作账号。');
    expect(screen.getByRole('checkbox', { name: '选择 工作账号' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '选择 Kimi' })).not.toBeChecked();
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
    state.trayFeedback = { message: '配置被其他程序修改，请确认恢复。', isError: true };
    act(() => notify());
    await screen.findByRole('dialog');
    expect(call).not.toHaveBeenCalledWith('set_enabled', { enabled: false, force: true });
    view.unmount();
    expect(stop).toHaveBeenCalledOnce();
  });
  it('does not configure or open Codex on mount', async () => {
    setup();
    render(<App />);
    await screen.findByRole('heading', { name: '我的配置' });
    expect(vi.mocked(call).mock.calls.every(([c]) => c === 'snapshot')).toBe(true);
  });
  it('lists profiles with multiple models and independently checks multiple providers', async () => {
    const state = setup([
      profile(),
      { ...profile('two', '智谱 GLM'), presetId: 'zhipu', models: ['glm-4.7'] },
    ]);
    render(<App />);
    fireEvent.click(await screen.findByRole('checkbox', { name: '选择 Kimi' }));
    await waitFor(() => expect(state.selectedProfiles).toEqual(['one']));
    fireEvent.click(screen.getByRole('checkbox', { name: '选择 智谱 GLM' }));
    await screen.findByText('已选 2 个配置 · 3 个模型');
    expect(state.selectedProfiles).toEqual(['one', 'two']);
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
    expect(call).not.toHaveBeenCalledWith('open_codex');
    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith('set_enabled', { enabled: true, force: false }),
    );
  });
  it('saving a multi-model profile returns home without selection or activation', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    expect(screen.getByLabelText('配置名称')).toHaveValue('Kimi');
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.7-code' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-only' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByText('「Kimi」已保存，包含 2 个模型。');
    expect(state.profiles).toHaveLength(1);
    expect(state.profiles[0].models).toEqual(['kimi-k3', 'kimi-k2.7-code']);
    expect(state.selectedProfiles).toEqual([]);
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
    expect(call).not.toHaveBeenCalledWith('open_codex');
  });
  it('numbers duplicate vendors and skips globally occupied names', async () => {
    setup([profile('one', '工作账号'), { ...profile('two', 'Kimi-2'), presetId: 'custom' }]);
    render(<App />);
    await addKimi();
    expect(screen.getByLabelText('配置名称')).toHaveValue('Kimi-3');
  });
  it('numbers the next profile after saving and reopening add', async () => {
    setup([]);
    render(<App />);
    await addKimi();
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    await addKimi();
    expect(screen.getByLabelText('配置名称')).toHaveValue('Kimi-2');
  });
  it('preserves user-entered name on vendor change and rejects case-insensitive duplicates', async () => {
    setup();
    render(<App />);
    await addKimi();
    fireEvent.change(screen.getByLabelText('配置名称'), { target: { value: ' kimi ' } });
    expect(screen.getByRole('button', { name: '保存配置' })).toBeDisabled();
    expect(screen.getByLabelText('配置名称')).toHaveAttribute('aria-invalid', 'true');
    fireEvent.click(screen.getByRole('button', { name: '智谱 GLM' }));
    expect(screen.getByLabelText('配置名称')).toHaveValue(' kimi ');
    fireEvent.submit(screen.getByRole('button', { name: '保存配置' }).closest('form')!);
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
  });
  it('edits all models under the same profile without duplicating the profile or requiring its saved key', async () => {
    const state = setup();
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByLabelText('模型厂商')).toHaveTextContent('Kimi');
    expect(screen.queryByRole('group', { name: '服务' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '智谱 GLM' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('API 地址')).toBeVisible();
    expect(screen.queryByLabelText('上下文长度')).not.toBeInTheDocument();
    expect(screen.getByLabelText('API Key')).not.toBeRequired();
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toBeChecked();
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k3' }));
    fireEvent.change(screen.getByLabelText('配置名称'), { target: { value: '工作账号' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByText('「工作账号」已保存，包含 1 个模型。');
    expect(state.profiles).toHaveLength(1);
    expect(state.profiles[0].id).toBe('one');
    expect(call).toHaveBeenCalledWith(
      'save_profile',
      expect.objectContaining({
        key: '',
        profile: expect.objectContaining({ presetId: 'kimi', models: ['kimi-k2.7-code'] }),
      }),
    );
  });
  it('requires a fresh key when changing the address', async () => {
    setup();
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic' } });
    fireEvent.change(screen.getByLabelText('API 地址'), {
      target: { value: 'https://other.example/v1' },
    });
    expect(screen.getByLabelText('API Key')).toHaveValue('');
    expect(screen.getByLabelText('API Key')).toBeRequired();
    fireEvent.submit(screen.getByRole('button', { name: '保存配置' }).closest('form')!);
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
  });
  it('persists the chosen default model when saving and reopening a profile', async () => {
    const state = setup();
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    fireEvent.click(screen.getByRole('button', { name: '将 kimi-k2.7-code 设为默认' }));
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByText('「Kimi」已保存，包含 2 个模型。');
    expect(state.profiles[0].models).toEqual(['kimi-k2.7-code', 'kimi-k3']);
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByRole('checkbox', { name: 'kimi-k2.7-code' })).toHaveAccessibleDescription(
      '默认',
    );
  });
  it('cannot submit no models and cannot apply an empty profile selection', async () => {
    setup();
    render(<App />);
    expect(await screen.findByRole('switch', { name: '开启 Codex Switch' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    for (const name of ['kimi-k3', 'kimi-k2.7-code'])
      fireEvent.click(screen.getByRole('checkbox', { name }));
    expect(screen.getByRole('button', { name: '保存配置' })).toBeDisabled();
  });
  it('failed validation retains the complete draft and does not activate', async () => {
    const state = setup();
    vi.mocked(call).mockImplementation(async (c) => {
      if (c === 'snapshot') return structuredClone(state) as never;
      throw new Error('模型不可用');
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toBeChecked();
    expect(screen.getByRole('heading', { name: '编辑配置' })).toBeInTheDocument();
  });
  it('tests only on explicit click, locks the form and supports cancellation without saving', async () => {
    const state = setup();
    let reject!: (error: Error) => void;
    vi.mocked(call).mockImplementation(async (command) => {
      if (command === 'snapshot') return structuredClone(state) as never;
      if (command === 'probe_endpoint')
        return new Promise((_, r) => {
          reject = r;
        }) as never;
      if (command === 'cancel_validation') reject(new Error('配置测试已取消'));
      return undefined as never;
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    expect(call).not.toHaveBeenCalledWith('probe_endpoint', expect.anything());
    fireEvent.click(screen.getByRole('button', { name: '测试配置' }));
    await screen.findByRole('button', { name: '取消测试' });
    expect(screen.getByRole('button', { name: '返回' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '保存配置' })).toBeDisabled();
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '取消测试' }));
    await screen.findByRole('alert');
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('manual testing keeps the editor open and saving does not trigger another probe', async () => {
    setup();
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByRole('switch', { name: '完整 URL' })).not.toBeChecked();
    expect(
      screen.queryByText('验证会发送少量测试请求，可能产生少量 API 费用。'),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '测试配置' }));
    await screen.findByText('配置测试通过 · 10 ms');
    expect(screen.getByRole('heading', { name: '编辑配置' })).toBeInTheDocument();
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
    vi.mocked(call).mockClear();
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByText('「Kimi」已保存，包含 2 个模型。');
    expect(call).toHaveBeenCalledWith(
      'save_profile',
      expect.objectContaining({ profile: expect.anything() }),
    );
    expect(call).not.toHaveBeenCalledWith('probe_endpoint', expect.anything());
    expect(call).not.toHaveBeenCalledWith('open_codex');
  });
  it('applies enabled selection changes immediately and prevents deselecting the last profile', async () => {
    const state = setup([profile(), profile('two', '工作账号')]);
    state.enabled = state.routing = true;
    state.selectedProfiles = ['one'];
    state.needsApply = false;
    render(<App />);
    expect(await screen.findByRole('checkbox', { name: '选择 Kimi' })).toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox', { name: '选择 工作账号' }));
    await screen.findByText('配置已切换，新旧会话的后续请求使用当前配置。');
    expect(state.selectedProfiles).toEqual(['one', 'two']);
    expect(screen.getByRole('checkbox', { name: '选择 Kimi' })).not.toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox', { name: '选择 Kimi' }));
    await waitFor(() => expect(state.selectedProfiles).toEqual(['two']));
    expect(screen.getByRole('checkbox', { name: '选择 工作账号' })).toBeDisabled();
    expect(screen.queryByText('待应用')).not.toBeInTheDocument();
    expect(state.needsApply).toBe(false);
  });
  it('locks profile creation editing and deletion while enabled with an explanation', async () => {
    const state = setup();
    state.enabled = true;
    state.selectedProfiles = ['one'];
    render(<App />);
    expect(await screen.findByRole('button', { name: '编辑 Kimi' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '删除 Kimi' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '新增配置' })).toBeDisabled();
    expect(screen.getByText('切换立即生效，至少保留一个配置；编辑请先关闭。')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: '编辑 Kimi' })).not.toBeDisabled(),
    );
  });
  it('locks an already open editor when the tray enables the switch', async () => {
    const state = setup();
    let notify = () => {};
    vi.mocked(subscribeToTray).mockImplementationOnce(async (cb) => {
      notify = cb;
      return () => {};
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    state.enabled = true;
    state.selectedProfiles = ['one'];
    act(() => notify());
    await screen.findByText('Codex Switch 已开启，请先关闭后再保存或编辑配置。');
    expect(screen.getByLabelText('配置名称')).toBeDisabled();
    expect(screen.getByRole('button', { name: '保存配置' })).toBeDisabled();
    fireEvent.submit(screen.getByRole('button', { name: '保存配置' }).closest('form')!);
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
    expect(screen.getByRole('button', { name: '返回' })).not.toBeDisabled();
  });
  it('keeps the active selection when switching fails', async () => {
    const state = setup([profile(), profile('two', '工作账号')]);
    state.enabled = state.routing = true;
    state.selectedProfiles = ['one'];
    state.needsApply = false;
    const originalCall = vi.mocked(call).getMockImplementation()!;
    vi.mocked(call).mockImplementation(async (command, args) => {
      if (command === 'select_profiles') throw new Error('密钥不可用');
      return originalCall(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('checkbox', { name: '选择 工作账号' }));
    await screen.findByText(/密钥不可用/);
    expect(state.selectedProfiles).toEqual(['one']);
    expect(screen.getByRole('checkbox', { name: '选择 Kimi' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '选择 工作账号' })).not.toBeChecked();
  });
  it('only allows opening Codex while enabled, including after disabling with a pending reload', async () => {
    const state = setup();
    state.selectedProfiles = ['one'];
    render(<App />);
    const open = await screen.findByRole('button', { name: '打开 Codex' });
    expect(open).toBeDisabled();
    fireEvent.click(open);
    expect(call).not.toHaveBeenCalledWith('open_codex');

    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() => expect(open).toBeEnabled());
    fireEvent.click(open);
    await screen.findByText('已打开 Codex。');
    expect(call).toHaveBeenCalledWith('open_codex');

    state.app.running = true;
    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    const reopen = await screen.findByRole('button', { name: '重新打开 Codex' });
    expect(reopen).toBeDisabled();
    fireEvent.click(reopen);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(call).not.toHaveBeenCalledWith('restart_codex');
  });
  it('keeps restart explicit after applying to a running Codex', async () => {
    const state = setup();
    state.selectedProfiles = ['one'];
    state.app.running = true;
    render(<App />);
    fireEvent.click(await screen.findByRole('switch', { name: '开启 Codex Switch' }));
    const open = await screen.findByRole('button', { name: '重新打开 Codex' });
    expect(call).not.toHaveBeenCalledWith('restart_codex');
    fireEvent.click(open);
    await screen.findByRole('dialog');
    expect(call).not.toHaveBeenCalledWith('restart_codex');
  });
  it('restores configuration through the off switch', async () => {
    const state = setup();
    state.enabled = true;
    state.routing = true;
    render(<App />);
    fireEvent.click(await screen.findByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith('set_enabled', { enabled: false, force: false }),
    );
  });
  it('restore conflict stays enabled and displays confirmation', async () => {
    const state = setup();
    state.enabled = true;
    vi.mocked(call).mockImplementation(async (c) => {
      if (c === 'snapshot') return structuredClone(state) as never;
      throw new Error('配置被其他程序修改');
    });
    render(<App />);
    const toggle = await screen.findByRole('switch', { name: '开启 Codex Switch' });
    fireEvent.click(toggle);
    await screen.findByRole('dialog');
    expect(toggle).toBeChecked();
  });
  it('deletes one configuration without touching another or silently applying', async () => {
    const state = setup([profile(), profile('two', 'Kimi-2')]);
    state.selectedProfiles = ['one', 'two'];
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '删除 Kimi' }));
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: '删除配置' }));
    await waitFor(() => expect(state.profiles).toHaveLength(1));
    expect(state.selectedProfiles).toEqual(['two']);
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('shows failed deletion inside its modal', async () => {
    const state = setup();
    vi.mocked(call).mockImplementation(async (c) => {
      if (c === 'snapshot') return structuredClone(state) as never;
      throw new Error('数据库不可用');
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '删除 Kimi' }));
    fireEvent.click(screen.getByRole('button', { name: '删除配置' }));
    await waitFor(() =>
      expect(within(screen.getByRole('dialog')).getByRole('alert')).toHaveTextContent(
        '数据库不可用',
      ),
    );
  });
  it('supports a named relay with custom multiple models', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    fireEvent.click(screen.getByRole('button', { name: 'coding plan' }));
    fireEvent.change(screen.getByLabelText('API 地址'), {
      target: { value: 'https://relay.example/v1' },
    });
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic' } });
    for (const model of ['a', 'b']) {
      fireEvent.change(screen.getByLabelText('添加自定义模型'), { target: { value: model } });
      fireEvent.click(screen.getByRole('button', { name: '添加模型' }));
    }
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].models).toEqual(['a', 'b']);
    expect(state.profiles[0].name).toBe('coding_plan');
  });
});

describe('CC Switch compatible configuration', () => {
  beforeEach(() => vi.clearAllMocks());
  it('loads native defaults while preserving a custom address and per-model override after editing', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    expect(screen.getByRole('combobox', { name: '接口格式' })).toHaveTextContent('Responses');
    fireEvent.change(screen.getByLabelText('API 地址'), {
      target: { value: 'https://relay.example/v1' },
    });
    expect(screen.queryByText('逐模型能力')).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/候选地址/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '编辑 kimi-k2.6 能力' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    fireEvent.change(screen.getByLabelText('kimi-k3 上下文长度'), { target: { value: '65536' } });
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-key' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByText('「Kimi」已保存，包含 1 个模型。');
    expect(state.profiles[0].options?.fullUrl ?? false).toBe(false);
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k3']?.contextWindow).toBe(65536);
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByLabelText('API 地址')).toHaveValue('https://relay.example/v1');
    expect(screen.getByRole('switch', { name: '完整 URL' })).not.toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    expect(screen.getByLabelText('kimi-k3 上下文长度')).toHaveValue(65536);
  });
  it('keeps model capability drafts isolated and discards cancelled edits', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.6' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    const initialContext = (screen.getByLabelText('kimi-k3 上下文长度') as HTMLInputElement).value;
    fireEvent.change(screen.getByLabelText('kimi-k3 上下文长度'), { target: { value: '65536' } });
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: '取消' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    expect((screen.getByLabelText('kimi-k3 上下文长度') as HTMLInputElement).value).toBe(
      initialContext,
    );
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: '取消' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k2.6 能力' }));
    fireEvent.change(screen.getByLabelText('kimi-k2.6 上下文长度'), {
      target: { value: '131072' },
    });
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-key' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].models).toEqual(['kimi-k3', 'kimi-k2.6']);
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k2.6']?.contextWindow).toBe(131072);
    expect(
      String(state.profiles[0].options?.modelOverrides?.['kimi-k3']?.contextWindow ?? ''),
    ).toBe(initialContext);
  });
  it('keeps vendor defaults stable and saves independent model request overrides', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.6' }));
    expect(screen.getByRole('combobox', { name: '接口格式' })).toHaveTextContent('Responses');
    expect(screen.getByLabelText('API 地址')).toHaveValue('https://api.moonshot.cn/v1');
    expect(screen.queryByText(/按当前地址和全部已选模型/)).not.toBeInTheDocument();
    expect(screen.queryByLabelText('上下文长度')).not.toBeInTheDocument();
    expect(screen.getByLabelText('API 地址').closest('.request-address-row')).toContainElement(
      screen.getByRole('combobox', { name: '接口格式' }),
    );
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k2.6 能力' }));
    fireEvent.change(screen.getByLabelText('kimi-k2.6 API 地址'), {
      target: { value: 'https://relay.example/custom/inference?version=2' },
    });
    fireEvent.click(screen.getByRole('switch', { name: 'kimi-k2.6 完整 URL' }));
    await act(async () => {
      fireEvent.keyDown(screen.getByRole('combobox', { name: 'kimi-k2.6 接口格式' }), {
        key: 'ArrowDown',
      });
    });
    await act(async () => {
      fireEvent.keyDown(
        within(screen.getByRole('listbox')).getByRole('option', { name: 'Chat Completions' }),
        { key: 'Enter' },
      );
    });
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    expect(screen.getByRole('combobox', { name: '接口格式' })).toHaveTextContent('Responses');
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-key' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].protocol).toBe('responses');
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k2.6']).toMatchObject({
      endpoint: 'https://relay.example/custom/inference?version=2',
      protocol: 'chat',
      fullUrl: true,
    });
    expect(call).not.toHaveBeenCalledWith('probe_endpoint', expect.anything());
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k2.6 能力' }));
    expect(screen.getByLabelText('kimi-k2.6 API 地址')).toHaveValue(
      'https://relay.example/custom/inference?version=2',
    );
    expect(screen.getByRole('combobox', { name: 'kimi-k2.6 接口格式' })).toHaveTextContent(
      'Chat Completions',
    );
    fireEvent.click(screen.getByRole('button', { name: '继承外层设置' }));
    expect(screen.getByLabelText('kimi-k2.6 API 地址')).toHaveValue('');
    expect(screen.getByRole('combobox', { name: 'kimi-k2.6 接口格式' })).toHaveTextContent(
      '继承（Responses）',
    );
    expect(screen.getByRole('switch', { name: 'kimi-k2.6 完整 URL' })).not.toBeChecked();
  });
  it('persists the exact outer full URL and requires a key when a model changes its saved address', async () => {
    const state = setup();
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 Kimi' }));
    fireEvent.click(screen.getByRole('switch', { name: '完整 URL' }));
    fireEvent.change(screen.getByLabelText('API 地址'), {
      target: { value: 'https://relay.example/invoke?version=2' },
    });
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-key' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].endpoint).toBe('https://relay.example/invoke?version=2');
    expect(state.profiles[0].options?.fullUrl).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByRole('switch', { name: '完整 URL' })).toBeChecked();
    expect(screen.getByLabelText('API Key')).not.toBeRequired();
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    fireEvent.change(screen.getByLabelText('kimi-k3 API 地址'), {
      target: { value: 'https://other.example/invoke' },
    });
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    expect(screen.getByLabelText('API Key')).toBeRequired();
    expect(call).not.toHaveBeenCalledWith('probe_endpoint', expect.anything());
  });
  it('automatically saves compaction in both directions and restores it after reopening', async () => {
    const state = setup();
    let view = render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '设置' }));
    expect(screen.queryByRole('switch', { name: '启用备用队列' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '保存路由设置' })).not.toBeInTheDocument();
    expect(screen.getByRole('switch', { name: '远程上下文压缩' })).not.toBeChecked();
    for (const enabled of [true, false]) {
      fireEvent.click(screen.getByRole('switch', { name: '远程上下文压缩' }));
      await screen.findByText('上下文压缩设置已自动保存，下次开启时生效。');
      await waitFor(() =>
        expect(screen.getByRole('switch', { name: '远程上下文压缩' })).toBeEnabled(),
      );
      expect(call).toHaveBeenCalledWith('save_routing_settings', {
        settings: { remoteCompaction: enabled },
      });
      expect(state.routingSettings?.remoteCompaction).toBe(enabled);
      view.unmount();
      view = render(<App />);
      fireEvent.click(await screen.findByRole('button', { name: '设置' }));
      expect(screen.getByRole('switch', { name: '远程上下文压缩' })).toHaveAttribute(
        'aria-checked',
        String(enabled),
      );
    }
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('keeps the saved compaction state when automatic saving fails', async () => {
    const state = setup();
    state.routingSettings = { remoteCompaction: true };
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '设置' }));
    vi.mocked(call).mockRejectedValueOnce(new Error('无法保存设置'));
    fireEvent.click(screen.getByRole('switch', { name: '远程上下文压缩' }));
    await screen.findByText('Error: 无法保存设置');
    await waitFor(() =>
      expect(screen.getByRole('switch', { name: '远程上下文压缩' })).toBeEnabled(),
    );
    expect(screen.getByRole('switch', { name: '远程上下文压缩' })).toBeChecked();
    expect(state.routingSettings?.remoteCompaction).toBe(true);
    expect(
      screen.queryByText('上下文压缩设置已自动保存，下次开启时生效。'),
    ).not.toBeInTheDocument();
  });
});
