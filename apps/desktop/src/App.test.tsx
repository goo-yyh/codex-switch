import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, act, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import {
  call,
  subscribeToAppChanges,
  type Snapshot,
  type Profile,
  type Preset,
} from './api/bridge';
import registry from '../../../packages/provider-registry/providers.json';
vi.mock('./api/bridge', () => ({
  isPreview: true,
  call: vi.fn(),
  subscribeToAppChanges: vi.fn(async () => () => {}),
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
    enabled: false,
    routing: false,
    pendingReload: false,
    activeRequests: 0,
    app: { installed: true, running: false },
    configPath: '/temporary/config.toml',
    autostart: false,
  };
  vi.mocked(call).mockImplementation(async (command, args) => {
    if (command === 'set_locale') state.locale = args?.locale as Snapshot['locale'];
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
      if (state.enabled) throw new Error('请先关闭服务，再修改配置或设置。');
      state.selectedProfiles = ids;
    }
    if (command === 'set_enabled') {
      state.enabled = Boolean(args?.enabled);
      state.routing = state.enabled;
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
  it('loads once and ignores window focus while still refreshing app state changes', async () => {
    const state = setup();
    let notify = () => {};
    vi.mocked(subscribeToAppChanges).mockImplementationOnce(async (cb) => {
      notify = cb;
      return () => {};
    });
    render(<App />);
    await screen.findByRole('heading', { name: '我的配置' });
    const snapshots = () =>
      vi.mocked(call).mock.calls.filter(([command]) => command === 'snapshot');
    expect(snapshots()).toHaveLength(1);
    await act(async () => {
      for (let i = 0; i < 5; i++) {
        fireEvent.blur(window);
        fireEvent.focus(window);
      }
    });
    expect(snapshots()).toHaveLength(1);
    state.selectedProfiles = ['one'];
    act(() => notify());
    await waitFor(() => expect(screen.getByLabelText('选择 Kimi')).toBeChecked());
    expect(snapshots()).toHaveLength(2);
  });

  it('refreshes remote candidates without replacing an unsaved form and saves defaults only on selection', async () => {
    const state = setup([]);
    state.presets = structuredClone(state.presets);
    let notify = () => {};
    vi.mocked(subscribeToAppChanges).mockImplementationOnce(async (cb) => {
      notify = cb;
      return () => {};
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '新增配置' }));
    fireEvent.change(screen.getByLabelText('配置名称'), { target: { value: '我的草稿' } });
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'example-only' } });
    const preset = state.presets[0];
    const defaults = {
      contextWindow: 131072,
      reasoningLevels: ['high'],
      defaultReasoningLevel: 'high',
    };
    preset.options!.modelOverrides!['test-remote-new'] = defaults;
    preset.variants![0].options!.modelOverrides!['test-remote-new'] = defaults;
    state.modelCandidates = { [preset.id]: ['test-remote-new', preset.model] };
    act(() => notify());
    const choice = await screen.findByRole('checkbox', { name: 'test-remote-new' });
    expect(choice).not.toBeChecked();
    expect(screen.getByLabelText('配置名称')).toHaveValue('我的草稿');
    expect(screen.getByLabelText('API Key')).toHaveValue('example-only');
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
    fireEvent.click(choice);
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].options?.modelOverrides?.['test-remote-new']).toEqual(defaults);
    expect(state.profiles[0].models[0]).toBe(preset.model);
    expect(state.enabled).toBe(false);
  });

  beforeEach(() => vi.clearAllMocks());
  it('switches the whole interface and docs language without changing saved user data', async () => {
    const state = setup([profile('one', '我的工作账号')]);
    const view = render(<App />);
    const language = await screen.findByRole('button', { name: '切换到英文' });
    const nav = screen.getByRole('navigation', { name: '应用导航' });
    expect(within(nav).getAllByRole('button')[0]).toBe(language);
    fireEvent.click(language);
    await screen.findByRole('heading', { name: 'My configurations' });
    expect(document.documentElement.lang).toBe('en');
    expect(screen.getByText('我的工作账号')).toBeInTheDocument();
    expect(state.profiles[0].models).toEqual(['kimi-k3', 'kimi-k2.7-code']);
    fireEvent.click(screen.getByRole('button', { name: 'Documentation' }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith('open_link', {
        url: expect.stringContaining('/content/docs/en/docs/quickstart.md'),
      }),
    );
    view.unmount();
    render(<App />);
    await screen.findByRole('heading', { name: 'My configurations' });
    fireEvent.click(screen.getByRole('button', { name: 'Switch to Chinese' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.locale).toBe('zh-CN');
    expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('preserves an unsaved form while translating providers, fields and model capabilities', async () => {
    setup([]);
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '新增配置' }));
    fireEvent.change(screen.getByLabelText('配置名称'), { target: { value: '保留这个名称' } });
    fireEvent.change(screen.getByLabelText('API Key'), {
      target: { value: 'example-not-a-real-key' },
    });
    fireEvent.click(screen.getByRole('button', { name: '切换到英文' }));
    await screen.findByRole('heading', { name: 'Add configuration' });
    expect(screen.getByLabelText('Configuration name')).toHaveValue('保留这个名称');
    expect(screen.getByLabelText('API Key')).toHaveValue('example-not-a-real-key');
    expect(screen.getByRole('button', { name: 'Zhipu GLM' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Qwen' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit glm-5.3 capabilities' }));
    expect(
      await screen.findByRole('dialog', { name: 'Edit glm-5.3 capabilities' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('glm-5.3 context window')).toBeInTheDocument();
    expect(screen.getByText('Default reasoning level')).toBeInTheDocument();
  });
  it('translates settings and keeps language selection locked while enabled', async () => {
    const state = setup();
    state.locale = 'en';
    state.selectedProfiles = ['one'];
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: 'Settings' }));
    await screen.findByRole('heading', { name: 'General settings' });
    expect(screen.getByRole('switch', { name: 'Remote context compaction' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    fireEvent.click(screen.getByRole('switch', { name: 'Enable Codex Switch' }));
    await screen.findByRole('region', { name: 'Connection controls' });
    expect(screen.getByLabelText('Switch to Chinese')).toBeDisabled();
    expect(screen.getByRole('switch', { name: 'Enable Codex Switch' })).toBeChecked();
  });
  it('keeps the current language when preference persistence fails', async () => {
    setup();
    const invoke = vi.mocked(call).getMockImplementation()!;
    vi.mocked(call).mockImplementation(async (command, args) => {
      if (command === 'set_locale') throw new Error('存储不可用。');
      return invoke(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '切换到英文' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('heading', { name: '我的配置' })).toBeInTheDocument();
  });

  it('refreshes selection and feedback after tray actions without applying again', async () => {
    const state = setup([profile(), profile('two', '工作账号')]);
    let notify = () => {};
    const stop = vi.fn();
    vi.mocked(subscribeToAppChanges).mockImplementationOnce(async (cb) => {
      notify = cb;
      return stop;
    });
    const view = render(<App />);
    await screen.findByRole('heading', { name: '我的配置' });
    state.selectedProfiles = ['two'];
    state.enabled = state.routing = true;
    state.trayFeedback = { message: '已从托盘切换到工作账号。', isError: false };
    act(() => notify());
    await screen.findByRole('region', { name: '连接控制' });
    expect(screen.queryByText('已从托盘切换到工作账号。')).not.toBeInTheDocument();
    expect(screen.getByLabelText('选择 工作账号')).toBeChecked();
    expect(screen.getByLabelText('选择 Kimi')).not.toBeChecked();
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
    state.trayFeedback = { message: '配置被其他程序修改，请重试。', isError: true };
    act(() => notify());
    expect(await screen.findByRole('alert')).toHaveTextContent('配置被其他程序修改');
    expect(call).not.toHaveBeenCalledWith('set_enabled', { enabled: false });
    state.enabled = false;
    state.trayFeedback = undefined;
    act(() => notify());
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByRole('button', { name: '设置' })).toBeEnabled();
    view.unmount();
    expect(stop).toHaveBeenCalledOnce();
  });
  it('preserves an action failure when refresh also consumes old tray success feedback', async () => {
    const state = setup();
    state.selectedProfiles = ['one'];
    const invoke = vi.mocked(call).getMockImplementation()!;
    vi.mocked(call).mockImplementation(async (command, args) => {
      if (command === 'set_enabled') {
        state.trayFeedback = { message: '旧操作成功', isError: false };
        throw new Error('配置被其他程序修改');
      }
      return invoke(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('switch', { name: '开启 Codex Switch' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('配置被其他程序修改');
    expect(screen.getByRole('switch', { name: '开启 Codex Switch' })).not.toBeChecked();
  });
  it('does not configure or open Codex on mount', async () => {
    setup();
    render(<App />);
    await screen.findByRole('heading', { name: '我的配置' });
    expect(
      vi.mocked(call).mock.calls.every(([c]) => c === 'snapshot' || c === 'check_update'),
    ).toBe(true);
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
    await waitFor(() => expect(call).toHaveBeenCalledWith('set_enabled', { enabled: true }));
  });
  it('saving a multi-model profile returns home without selection or activation', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    expect(screen.getByLabelText('配置名称')).toHaveValue('Kimi');
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.7-code' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic-only' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles).toHaveLength(1);
    expect(state.profiles[0].models).toEqual(['kimi-k3', 'kimi-k2.7-code']);
    expect(state.selectedProfiles).toEqual([]);
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
    expect(call).not.toHaveBeenCalledWith('open_codex');
  });
  it('orders providers and only offers a plan selector when there are distinct plans', async () => {
    setup([]);
    render(<App />);
    fireEvent.click((await screen.findAllByRole('button', { name: '新增配置' }))[0]);
    const providers = within(screen.getByRole('group', { name: '服务' }));
    const orderedNames = ['智谱 GLM', 'DeepSeek', 'Kimi', '千问', 'MiniMax', 'coding plan'];
    const buttons = providers.getAllByRole('button');
    expect(buttons).toHaveLength(orderedNames.length);
    orderedNames.forEach((name, index) => expect(buttons[index]).toHaveAccessibleName(name));
    expect(providers.getByRole('button', { name: '智谱 GLM' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    for (const name of ['DeepSeek']) {
      fireEvent.click(providers.getByRole('button', { name }));
      expect(screen.queryByRole('combobox', { name: '服务套餐' })).not.toBeInTheDocument();
      expect(screen.getByLabelText('API 地址')).toBeEnabled();
    }
    for (const name of ['智谱 GLM', 'Kimi', '千问', 'MiniMax']) {
      fireEvent.click(providers.getByRole('button', { name }));
      expect(screen.getByRole('combobox', { name: '服务套餐' })).toBeVisible();
    }
  });
  it.each([
    [
      '智谱 GLM',
      '智谱 Coding Plan',
      'https://open.bigmodel.cn/api/coding/paas/v4',
      'chat',
      'glm-5.3',
    ],
    ['MiniMax', 'MiniMax Token Plan', 'https://api.minimax.cn/v1', 'responses', 'MiniMax-M3'],
  ])(
    'loads and saves the %s subscription with its endpoint and protocol',
    async (provider, plan, endpoint, protocol, model) => {
      const state = setup([]);
      render(<App />);
      fireEvent.click((await screen.findAllByRole('button', { name: '新增配置' }))[0]);
      fireEvent.click(screen.getByRole('button', { name: provider }));
      fireEvent.change(screen.getByLabelText('API Key'), {
        target: { value: 'synthetic-standard-key' },
      });
      fireEvent.keyDown(screen.getByRole('combobox', { name: '服务套餐' }), { key: 'ArrowDown' });
      expect(await screen.findByRole('option', { name: plan })).toBeVisible();
      expect(screen.queryByRole('option', { name: '自定义地址或接口' })).not.toBeInTheDocument();
      fireEvent.click(screen.getByRole('option', { name: plan }));
      expect(screen.getByLabelText('API 地址')).toHaveValue(endpoint);
      expect(screen.getByLabelText('API Key')).toHaveValue('');
      expect(screen.getByRole('checkbox', { name: model })).toBeChecked();
      if (provider === '智谱 GLM') {
        expect(screen.getByRole('checkbox', { name: 'glm-5.3-flash' })).toBeVisible();
        expect(screen.queryByRole('checkbox', { name: 'glm-4.7' })).not.toBeInTheDocument();
      }
      fireEvent.change(screen.getByLabelText('API Key'), {
        target: { value: 'synthetic-plan-key' },
      });
      fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
      await screen.findByRole('heading', { name: '我的配置' });
      expect(state.profiles[0]).toMatchObject({ endpoint, protocol, models: [model] });
      fireEvent.click(screen.getByRole('button', { name: `编辑 ${provider}` }));
      expect(screen.getByRole('combobox', { name: '服务套餐' })).toHaveTextContent(plan);
      fireEvent.change(screen.getByLabelText('API 地址'), {
        target: { value: 'https://custom.example/v1' },
      });
      expect(screen.getByRole('combobox', { name: '服务套餐' })).toHaveTextContent('选择套餐');
      expect(screen.getByLabelText('API 地址')).toHaveValue('https://custom.example/v1');
    },
  );
  it('numbers duplicate vendors and skips globally occupied names', async () => {
    setup([profile('one', '工作账号'), { ...profile('two', 'Kimi-2'), presetId: 'custom' }]);
    render(<App />);
    await addKimi();
    expect(screen.getByLabelText('配置名称')).toHaveValue('Kimi-3');
  });
  it('keeps each model context at the 80% preset after editing and saving again', async () => {
    const state = setup([]);
    render(<App />);
    await addKimi();
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.6' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k2.6 能力' }));
    expect(screen.getByLabelText('kimi-k2.6 上下文长度')).toHaveValue(209715);
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: 'synthetic' } });
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].contextWindow).toBe(838860);
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k2.6']?.contextWindow).toBe(209715);
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k2.6 能力' }));
    expect(screen.getByLabelText('kimi-k2.6 上下文长度')).toHaveValue(209715);
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k2.6']?.contextWindow).toBe(209715);
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
    await screen.findByRole('heading', { name: '我的配置' });
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
    await screen.findByRole('heading', { name: '我的配置' });
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
    await screen.findByRole('heading', { name: '我的配置' });
    expect(call).toHaveBeenCalledWith(
      'save_profile',
      expect.objectContaining({ profile: expect.anything() }),
    );
    expect(call).not.toHaveBeenCalledWith('probe_endpoint', expect.anything());
    expect(call).not.toHaveBeenCalledWith('open_codex');
  });
  it('locks configuration while keeping launch and shutdown actions available', async () => {
    const user = userEvent.setup();
    const state = setup([profile(), profile('two', '工作账号')]);
    state.enabled = state.routing = true;
    state.selectedProfiles = ['one'];
    render(<App />);
    const dialog = await screen.findByRole('region', { name: '连接控制' });
    expect(screen.getAllByRole('switch')).toHaveLength(1);
    const stop = within(dialog).getByRole('switch', { name: '开启 Codex Switch' });
    expect(stop).toBeChecked();
    expect(screen.getByRole('button', { name: '设置' })).toBeDisabled();
    expect(screen.getByLabelText('选择 Kimi')).toBeDisabled();
    expect(screen.getByLabelText('选择 工作账号')).toBeDisabled();
    await user.keyboard('{Escape}');
    expect(dialog).toBeVisible();
    fireEvent.pointerDown(document.querySelector('.service-lock-overlay')!);
    fireEvent.click(document.querySelector('.service-lock-overlay')!);
    expect(dialog).toBeVisible();
    stop.focus();
    await user.tab();
    expect(within(dialog).getByRole('button', { name: '打开 Codex' })).toHaveFocus();
    expect(state.selectedProfiles).toEqual(['one']);
    expect(call).not.toHaveBeenCalledWith('select_profiles', expect.anything());
    await user.click(stop);
    await waitFor(() =>
      expect(document.querySelector('.service-lock-overlay')).not.toBeInTheDocument(),
    );
    expect(call).toHaveBeenCalledWith('set_enabled', { enabled: false });
    expect(screen.getByRole('button', { name: '设置' })).toBeEnabled();
    fireEvent.click(screen.getByRole('checkbox', { name: '选择 工作账号' }));
    await waitFor(() => expect(state.selectedProfiles).toEqual(['one', 'two']));
  });
  it.each(['settings', 'editor', 'model', 'delete'])(
    'locks an open %s when enabled from the tray',
    async (page) => {
      const state = setup();
      let notify = () => {};
      vi.mocked(subscribeToAppChanges).mockImplementationOnce(async (cb) => {
        notify = cb;
        return () => {};
      });
      render(<App />);
      await screen.findByRole('button', { name: '设置' });
      if (page === 'settings') {
        fireEvent.click(screen.getByRole('button', { name: '设置' }));
      } else if (page === 'delete') {
        fireEvent.click(screen.getByRole('button', { name: '删除 Kimi' }));
      } else {
        fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
        if (page === 'model')
          fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
      }
      state.enabled = true;
      state.selectedProfiles = ['one'];
      act(() => notify());
      await screen.findByRole('region', { name: '连接控制' });
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
      expect(screen.getAllByRole('switch')).toHaveLength(1);
      expect(call).not.toHaveBeenCalledWith('save_profile', expect.anything());
      expect(call).not.toHaveBeenCalledWith('set_autostart', expect.anything());
      fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
      await waitFor(() =>
        expect(document.querySelector('.service-lock-overlay')).not.toBeInTheDocument(),
      );
      await waitFor(() => expect(screen.getByRole('button', { name: '设置' })).toBeEnabled());
    },
  );
  it('opens Codex from the enabled control bar without unlocking configuration', async () => {
    const state = setup();
    state.enabled = state.routing = true;
    render(<App />);
    const dialog = await screen.findByRole('region', { name: '连接控制' });
    await userEvent.click(within(dialog).getByRole('button', { name: '打开 Codex' }));
    await waitFor(() => expect(call).toHaveBeenCalledWith('open_codex'));
    expect(dialog).toBeVisible();
    expect(document.querySelector('.configuration-area')).toHaveAttribute('inert');
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('confirms restart while enabled and returns to the locked control bar on cancel or success', async () => {
    const state = setup();
    state.enabled = state.routing = state.pendingReload = state.app.running = true;
    const original = vi.mocked(call).getMockImplementation()!;
    vi.mocked(call).mockImplementation(async (command, args) => {
      if (command === 'restart_codex') state.pendingReload = false;
      return original(command, args);
    });
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: '重新打开 Codex' }));
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    expect(document.querySelector('.configuration-area')).toHaveAttribute('inert');
    expect(call).not.toHaveBeenCalledWith('restart_codex');
    await user.click(screen.getByRole('button', { name: '稍后' }));
    await screen.findByRole('region', { name: '连接控制' });
    await user.click(screen.getByRole('button', { name: '重新打开 Codex' }));
    await user.click(screen.getByRole('button', { name: '正常重启并应用' }));
    await screen.findByRole('region', { name: '连接控制' });
    expect(call).toHaveBeenCalledWith('restart_codex');
    expect(screen.getByRole('button', { name: '打开 Codex' })).toBeEnabled();
    expect(state.enabled).toBe(true);
    expect(call).not.toHaveBeenCalledWith('set_enabled', expect.anything());
  });
  it('shows the lock after enabling without opening or restarting Codex', async () => {
    const state = setup();
    state.selectedProfiles = ['one'];
    state.app.running = true;
    render(<App />);
    fireEvent.click(await screen.findByRole('switch', { name: '开启 Codex Switch' }));
    await screen.findByRole('region', { name: '连接控制' });
    expect(call).not.toHaveBeenCalledWith('open_codex');
    expect(call).not.toHaveBeenCalledWith('restart_codex');
    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() =>
      expect(document.querySelector('.service-lock-overlay')).not.toBeInTheDocument(),
    );
    expect(screen.getByRole('switch', { name: '开启 Codex Switch' })).not.toBeChecked();
  });
  it('keeps the lock on restoration failure and allows retry without another confirmation', async () => {
    const state = setup();
    state.enabled = true;
    const originalCall = vi.mocked(call).getMockImplementation()!;
    let attempts = 0;
    vi.mocked(call).mockImplementation(async (command, args) => {
      if (command === 'set_enabled' && attempts++ === 0) throw new Error('恢复配置失败');
      return originalCall(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole('switch', { name: '开启 Codex Switch' }));
    const dialog = screen.getByRole('region', { name: '连接控制' });
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('恢复配置失败');
    expect(state.enabled).toBe(true);
    expect(within(dialog).getByRole('switch', { name: '开启 Codex Switch' })).toBeChecked();
    await waitFor(() =>
      expect(screen.getByRole('switch', { name: '开启 Codex Switch' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('switch', { name: '开启 Codex Switch' }));
    await waitFor(() =>
      expect(document.querySelector('.service-lock-overlay')).not.toBeInTheDocument(),
    );
    expect(state.enabled).toBe(false);
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
    await screen.findByRole('heading', { name: '我的配置' });
    expect(state.profiles[0].options?.fullUrl ?? false).toBe(false);
    expect(state.profiles[0].options?.modelOverrides?.['kimi-k3']?.contextWindow).toBe(65536);
    fireEvent.click(screen.getByRole('button', { name: '编辑 Kimi' }));
    expect(screen.getByLabelText('API 地址')).toHaveValue('https://relay.example/v1');
    expect(screen.getByRole('switch', { name: '完整 URL' })).not.toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: '编辑 kimi-k3 能力' }));
    expect(screen.getByLabelText('kimi-k3 上下文长度')).toHaveValue(65536);
  });
  it('shows official defaults for a saved GLM Flash profile with missing capabilities', async () => {
    const saved = {
      ...profile(),
      name: 'GLM',
      presetId: 'zhipu',
      endpoint: 'https://open.bigmodel.cn/api/v1',
      models: ['glm-5.3-flash'],
    };
    const state = setup([saved]);
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '编辑 GLM' }));
    fireEvent.click(screen.getByRole('button', { name: '编辑 glm-5.3-flash 能力' }));
    expect(screen.getByLabelText('glm-5.3-flash 思考档位（逗号分隔）')).toHaveValue('low,high,max');
    expect(screen.getByRole('combobox', { name: 'glm-5.3-flash 默认思考档位' })).toHaveTextContent(
      'max',
    );
    fireEvent.click(screen.getByRole('button', { name: '确认修改' }));
    fireEvent.click(screen.getByRole('button', { name: '保存配置' }));
    await screen.findByRole('heading', { name: '我的配置' });
    expect(
      state.profiles[0].options?.modelOverrides?.['glm-5.3-flash']?.defaultReasoningLevel,
    ).toBe('max');
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
