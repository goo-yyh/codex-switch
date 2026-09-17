import { useEffect, useRef, useState } from 'react';
import * as Switch from '@radix-ui/react-switch';
import {
  ArrowUpRight,
  ArrowLeft,
  ShieldCheck,
  Check,
  Eye,
  EyeOff,
  LoaderCircle,
  BookOpen,
} from 'lucide-react';
import {
  call,
  isPreview,
  subscribeToTray,
  type Snapshot,
  type Profile,
  type ModelOptions,
} from './bridge';
import { Picker, Modal } from './components';
import { CompatibilityFields, CompactionPreferences } from './CompatibilityFields';
import { ModelSelection } from './ModelSelection';
import { RequestFields } from './RequestFields';
import { ModelCapabilityFields } from './ModelCapabilityFields';
import { withReasoningDefaults } from './modelDefaults';
import { ConnectionWorkspace, ServiceMark } from './ConnectionWorkspace';
import { product, documentationUrl } from './product';
import brandIcon from '../../../packages/brand/mark.png';
const newConnection = (): Profile => ({
  id: '',
  name: '',
  presetId: 'deepseek',
  endpoint: '',
  models: [],
  protocol: 'responses',
  contextWindow: 32768,
});
export default function App() {
  const [data, setData] = useState<Snapshot>();
  const mainRef = useRef<HTMLElement>(null);
  const [view, setView] = useState<'home' | 'form' | 'settings'>('home');
  const [form, setForm] = useState<Profile>(newConnection);
  const [editingModel, setEditingModel] = useState<string | null>(null);
  const [modelDraft, setModelDraft] = useState<ModelOptions>({});
  const [key, setKey] = useState('');
  const models = form.models;
  const [nameEdited, setNameEdited] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [phase, setPhase] = useState<'idle' | 'working' | 'testing' | 'applying' | 'opening'>(
    'idle',
  );
  const busy = phase !== 'idle';
  const busyRef = useRef(false);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');
  const [modal, setModal] = useState<'restart' | 'edit-locked' | 'delete' | null>(null);
  const [deleteId, setDeleteId] = useState('');
  const [validationId, setValidationId] = useState<string | null>(null);
  useEffect(() => {
    if (data?.enabled) {
      setModal(null);
      setEditingModel(null);
      setKey('');
      setShowKey(false);
    }
  }, [data?.enabled]);
  useEffect(() => {
    if (mainRef.current) mainRef.current.scrollTop = 0;
  }, [view]);
  async function refresh() {
    const next = await call<Snapshot>('snapshot');
    setData(next);
    if (next.trayFeedback) {
      const { message, isError } = next.trayFeedback;
      setError(isError ? message : '');
    }
  }
  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    const onFocus = () => {
      if (!busyRef.current) refresh().catch((e) => setError(String(e)));
    };
    window.addEventListener('focus', onFocus);
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    subscribeToTray(onFocus)
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      unsubscribe?.();
      window.removeEventListener('focus', onFocus);
    };
  }, []);
  async function run(action: () => Promise<void>) {
    if (busyRef.current) return;
    busyRef.current = true;
    setPhase('working');
    setError('');
    setNotice('');
    try {
      await action();
      await refresh();
    } catch (e) {
      const text = String(e);
      setError(text);
      try {
        await refresh();
      } catch {
        /* Keep the original actionable error. */
      }
    } finally {
      busyRef.current = false;
      setPhase('idle');
    }
  }
  function nextName(base: string, presetId: string, excludeId = '') {
    const profiles = (data?.profiles ?? []).filter((p) => p.id !== excludeId);
    let n = profiles.filter((p) => p.presetId === presetId).length + 1;
    let name = n === 1 ? base : `${base}-${n}`;
    while (profiles.some((p) => p.name.toLowerCase() === name.toLowerCase()))
      name = `${base}-${++n}`;
    return name;
  }
  function usePreset(id: string) {
    if (form.id || id === form.presetId) return;
    setKey('');
    setShowKey(false);
    const p = data?.presets.find((p) => p.id === id);
    setForm((f) => ({
      ...f,
      presetId: id,
      name: nameEdited ? f.name : nextName(p?.name || 'coding_plan', id, form.id),
      endpoint: p?.endpoint || '',
      models: p ? [p.model] : [],
      protocol: p?.protocol || 'responses',
      contextWindow: p?.contextWindow || 32768,
      options: structuredClone(p?.options ?? {}),
    }));
  }
  function add() {
    if (data?.enabled) {
      setError('');
      setModal('edit-locked');
      return;
    }
    const p = data?.presets[0];
    setForm({
      ...newConnection(),
      presetId: p?.id || 'custom',
      name: nextName(p?.name || 'coding_plan', p?.id || 'custom'),
      endpoint: p?.endpoint || '',
      models: p ? [p.model] : [],
      protocol: p?.protocol || 'responses',
      contextWindow: p?.contextWindow || 32768,
      options: structuredClone(p?.options ?? {}),
    });
    setNameEdited(false);
    setKey('');
    setShowKey(false);
    setError('');
    setNotice('');
    setView('form');
  }
  async function toggle(value: boolean) {
    await run(async () => {
      await call('set_enabled', {
        enabled: value,
      });
    });
  }
  async function save() {
    if (busyRef.current) return;
    if (data?.enabled) {
      setError('');
      setModal('edit-locked');
      return;
    }
    if (!form.name.trim()) {
      setError('请填写配置名称。');
      return;
    }
    if (nameConflict) {
      setError('配置名称已存在，请换一个名称。');
      return;
    }
    if (!models.length) {
      setError('请至少选择一个模型。');
      return;
    }
    if (!canReuseKey && !key.trim()) {
      setError('请填写当前配置的 API Key。');
      return;
    }
    await run(async () => {
      const saved = await call<Profile>('save_profile', {
        profile: { ...form, name: form.name.trim() },
        key,
      });
      setForm(saved);
      setKey('');
      setShowKey(false);
      setView('home');
    });
  }
  async function testConfiguration() {
    if (busyRef.current || data?.enabled) return;
    if (!canReuseKey && !key.trim()) {
      setError('请填写当前配置的 API Key。');
      return;
    }
    const requestId = crypto.randomUUID();
    setValidationId(requestId);
    await run(async () => {
      setPhase('testing');
      const receipt = await call<import('./bridge').Receipt>('probe_endpoint', {
        profile: form,
        key,
        requestId,
      });
      if (!receipt.ok) throw new Error(receipt.message);
      setNotice(`${receipt.message} · ${receipt.elapsedMs} ms`);
    });
    setValidationId(null);
  }
  const preset = data?.presets.find((p) => p.id === form.presetId);
  const variantIndex =
    preset?.variants?.findIndex(
      (variant) => variant.endpoint === form.endpoint && variant.protocol === form.protocol,
    ) ?? -1;
  const packageVariant = preset?.variants
    ?.slice(1)
    .find((variant) => variant.endpoint === form.endpoint);
  const original = data?.profiles.find((c) => c.id === form.id);
  const normalizeScope = (endpoint: string, fullUrl = false) => {
    try {
      const u = new URL(endpoint.trim());
      if (fullUrl) return u.href;
      u.pathname = u.pathname
        .replace(/\/+$/, '')
        .replace(/\/(chat\/completions|responses|models)$/, '');
      return u.href.replace(/\/+$/, '');
    } catch {
      return null;
    }
  };
  const scope = normalizeScope(form.endpoint, form.options?.fullUrl);
  const canReuseKey = Boolean(
    original &&
    original.presetId === form.presetId &&
    scope &&
    normalizeScope(original.endpoint, original.options?.fullUrl) === scope &&
    Boolean(original.options?.fullUrl) === Boolean(form.options?.fullUrl) &&
    models.every((model) => {
      const before = original.options?.modelOverrides?.[model];
      const after = form.options?.modelOverrides?.[model];
      const oldFull = before?.fullUrl ?? Boolean(original.options?.fullUrl);
      const newFull = after?.fullUrl ?? Boolean(form.options?.fullUrl);
      return (
        oldFull === newFull &&
        normalizeScope(before?.endpoint || original.endpoint, oldFull) ===
          normalizeScope(after?.endpoint || form.endpoint, newFull)
      );
    }),
  );
  const nameConflict = Boolean(
    data?.profiles.some(
      (p) => p.id !== form.id && p.name.toLowerCase() === form.name.trim().toLowerCase(),
    ),
  );
  const phaseLabel = {
    idle: '保存配置',
    working: '处理中…',
    testing: '保存配置',
    applying: '正在应用配置…',
    opening: '正在打开 Codex…',
  }[phase];
  if (!data)
    return (
      <div className="loading">
        <LoaderCircle className="spin" />
        <p>{error || '正在准备 Codex Switch…'}</p>
      </div>
    );
  return (
    <div className="app-shell">
      <header className="app-header" inert={data.enabled}>
        <button
          className="brand"
          disabled={busy}
          aria-label="回到首页"
          onClick={() => {
            setView('home');
            setKey('');
            setShowKey(false);
            setError('');
            setNotice('');
          }}
        >
          <img className="brand-symbol" src={brandIcon} alt="" />
          <span>Codex Switch</span>
        </button>
        <nav className="header-actions" aria-label="应用导航">
          <button
            className="header-link"
            disabled={busy}
            aria-label="使用文档"
            onClick={() =>
              run(async () => {
                await call('open_link', { url: documentationUrl() });
              })
            }
          >
            <BookOpen size={16} />
            <span>文档</span>
          </button>
        </nav>
      </header>
      <div className="app-content" inert={data.enabled}>
        {isPreview && (
          <div className="preview-bar">交互预览 · 数据仅保存在本页，不会修改配置或调用服务</div>
        )}
        <main ref={mainRef}>
          {view === 'home' && (
            <ConnectionWorkspace
              data={data}
              busy={busy}
              onAdd={() => add()}
              onSelect={(ids) =>
                run(async () => {
                  if (data.enabled) {
                    throw new Error('请先关闭服务，再修改配置或设置。');
                  }
                  await call('select_profiles', { ids });
                })
              }
              onEdit={(c) => {
                if (data.enabled) {
                  setError('');
                  setModal('edit-locked');
                  return;
                }
                setForm(c);
                setNameEdited(true);
                setKey('');
                setShowKey(false);
                setError('');
                setNotice('');
                setView('form');
              }}
              onDelete={(c) => {
                if (data.enabled) {
                  setError('');
                  setModal('edit-locked');
                  return;
                }
                setError('');
                setDeleteId(c.id);
                setModal('delete');
              }}
              onToggle={toggle}
              onSettings={() => {
                setView('settings');
                setKey('');
                setShowKey(false);
                setError('');
                setNotice('');
              }}
              onRecover={() =>
                run(async () => {
                  await call('recover_connection');
                })
              }
              onOpen={() => {
                if (data.pendingReload) {
                  setError('');
                  setModal('restart');
                } else
                  run(async () => {
                    if (!data.app.installed) {
                      await call('open_link', { url: 'https://chatgpt.com/download/' });
                      return;
                    }
                    await call('open_codex');
                  });
              }}
            />
          )}
          {view === 'form' && (
            <section className="form-page">
              <div className="form-titlebar">
                <button
                  className="text-button back"
                  disabled={busy}
                  onClick={() => {
                    setView('home');
                    setKey('');
                    setShowKey(false);
                  }}
                >
                  <ArrowLeft size={15} />
                  返回
                </button>
                <h1>{form.id ? '编辑配置' : '新增配置'}</h1>
              </div>
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  save();
                }}
              >
                {data.enabled && (
                  <p className="inline-note" role="status">
                    Codex Switch 已开启，请先关闭后再保存或编辑配置。
                  </p>
                )}
                <fieldset className="form-fields" disabled={busy || data.enabled}>
                  <section className="form-card" aria-labelledby="connection-card-title">
                    <div className="form-card-heading">
                      <h2 id="connection-card-title">连接信息</h2>
                      {form.id && (
                        <div className="provider-identity" aria-label="模型厂商">
                          <ServiceMark
                            name={preset?.name || 'coding plan'}
                            presetId={form.presetId}
                          />
                          <strong>{preset?.name || 'coding plan'}</strong>
                        </div>
                      )}
                    </div>
                    {!form.id && (
                      <div className="provider-options" role="group" aria-label="服务">
                        {[...data.presets, { id: 'custom', name: 'coding plan' }].map((p) => (
                          <button
                            className={`provider-option ${form.presetId === p.id ? 'selected' : ''}`}
                            aria-pressed={form.presetId === p.id}
                            type="button"
                            disabled={busy || data.enabled}
                            onClick={() => usePreset(p.id)}
                            key={p.id}
                          >
                            <ServiceMark name={p.name} presetId={p.id} />
                            <span>{p.name}</span>
                            {form.presetId === p.id && <Check size={13} />}
                          </button>
                        ))}
                      </div>
                    )}
                    {preset?.variants && preset.variants.length > 1 && (
                      <label className="field">
                        服务套餐
                        <Picker
                          label="服务套餐"
                          disabled={busy || data.enabled}
                          value={variantIndex < 0 ? '' : String(variantIndex)}
                          placeholder="选择套餐"
                          options={preset.variants.map((v, i) => ({
                            value: String(i),
                            label: v.name,
                          }))}
                          onChange={(value) => {
                            const v = preset.variants?.[Number(value)];
                            if (v) {
                              setKey('');
                              setForm({
                                ...form,
                                endpoint: v.endpoint,
                                models: [v.model],
                                protocol: v.protocol,
                                contextWindow: v.contextWindow,
                                options: structuredClone(v.options ?? {}),
                              });
                            }
                          }}
                        />
                      </label>
                    )}
                    <div className="connection-fields">
                      {
                        <label className="field">
                          配置名称
                          <input
                            className="input"
                            maxLength={120}
                            value={form.name}
                            onChange={(e) => {
                              setNameEdited(true);
                              setForm({ ...form, name: e.target.value });
                            }}
                            aria-invalid={nameConflict}
                            aria-describedby={nameConflict ? 'name-error' : undefined}
                            required
                            placeholder="例如：coding_plan"
                          />
                        </label>
                      }
                      {nameConflict && (
                        <p className="field-hint" id="name-error" role="alert">
                          配置名称已存在，请换一个名称。
                        </p>
                      )}
                      <RequestFields
                        endpoint={form.endpoint}
                        protocol={form.protocol}
                        fullUrl={Boolean(form.options?.fullUrl)}
                        disabled={busy || data.enabled}
                        onChange={(change) => {
                          if ('endpoint' in change || 'fullUrl' in change) setKey('');
                          setForm({
                            ...form,
                            endpoint: change.endpoint ?? form.endpoint,
                            protocol: change.protocol ?? form.protocol,
                            options: {
                              ...form.options,
                              fullUrl: change.fullUrl ?? form.options?.fullUrl,
                            },
                          });
                        }}
                      />
                      <div className="field">
                        <div className="label-line">
                          <label htmlFor="key">API Key</label>
                          {preset && (
                            <button
                              type="button"
                              className="text-button small"
                              onClick={() => call('open_link', { url: preset.keyUrl })}
                            >
                              获取密钥
                              <ArrowUpRight size={13} />
                            </button>
                          )}
                        </div>
                        <div className="key-input">
                          <input
                            id="key"
                            className="input"
                            type={showKey ? 'text' : 'password'}
                            value={key}
                            onChange={(e) => setKey(e.target.value)}
                            placeholder={
                              canReuseKey ? '已保存，留空保留原密钥' : '粘贴当前服务的密钥'
                            }
                            autoComplete="off"
                            spellCheck={false}
                            required={!canReuseKey}
                          />
                          <button
                            type="button"
                            className="icon-button"
                            onClick={() => setShowKey(!showKey)}
                            aria-label={showKey ? '隐藏密钥' : '显示密钥'}
                          >
                            {showKey ? <EyeOff size={17} /> : <Eye size={17} />}
                          </button>
                        </div>
                      </div>
                      {original && !canReuseKey && (
                        <p className="field-hint">地址已改变，请填写当前服务的 Key。</p>
                      )}
                    </div>
                  </section>
                  <section className="form-card" aria-labelledby="models-label">
                    <ModelSelection
                      key={`${form.presetId}:${form.id}`}
                      provider={form.presetId}
                      availableModels={
                        packageVariant?.options?.modelOverrides
                          ? Object.keys(packageVariant.options.modelOverrides)
                          : undefined
                      }
                      multiple
                      models={models}
                      disabled={busy || data.enabled}
                      onEditModel={(model) => {
                        setModelDraft(withReasoningDefaults(form, model, preset));
                        setEditingModel(model);
                      }}
                      onChange={(values) => {
                        setForm({ ...form, models: values });
                      }}
                    />
                  </section>
                  {(form.protocol === 'chat' ||
                    models.some(
                      (model) => form.options?.modelOverrides?.[model]?.protocol === 'chat',
                    )) && (
                    <section className="form-card" aria-labelledby="advanced-card-title">
                      <div className="form-card-heading">
                        <h2 id="advanced-card-title">高级设置</h2>
                      </div>
                      <CompatibilityFields
                        form={form}
                        onChange={setForm}
                        disabled={busy || data.enabled}
                      />
                    </section>
                  )}
                </fieldset>
                <footer className="form-footer">
                  {error && !modal && (
                    <div className="feedback error" role="alert">
                      {error}
                    </div>
                  )}
                  {notice && (
                    <div className="feedback success" role="status">
                      {notice}
                    </div>
                  )}
                  <div className="form-actions">
                    {validationId && (
                      <button
                        className="text-button"
                        type="button"
                        onClick={() => {
                          call('cancel_validation', { requestId: validationId }).catch((e) =>
                            setError(String(e)),
                          );
                        }}
                      >
                        取消测试
                      </button>
                    )}
                    <button
                      className="secondary"
                      type="button"
                      disabled={
                        busy || data.enabled || nameConflict || !models.length || !form.endpoint
                      }
                      onClick={(event) => {
                        if (event.currentTarget.form?.reportValidity()) testConfiguration();
                      }}
                    >
                      {phase === 'testing' && <LoaderCircle className="spin" size={17} />}
                      {phase === 'testing' ? '测试中…' : '测试配置'}
                    </button>
                    <button
                      className="primary"
                      disabled={busy || data.enabled || nameConflict || !models.length}
                      type="submit"
                    >
                      {busy && phase !== 'testing' ? (
                        <LoaderCircle className="spin" size={17} />
                      ) : (
                        <ArrowUpRight size={17} />
                      )}
                      {phaseLabel}
                    </button>
                  </div>
                </footer>
              </form>
            </section>
          )}
          {view === 'settings' && (
            <section className="settings-page">
              <button className="text-button back" disabled={busy} onClick={() => setView('home')}>
                <ArrowLeft size={15} />
                返回
              </button>
              <h1>通用设置</h1>
              <h2 className="settings-section-title">应用设置</h2>
              <div className="settings-group">
                <CompactionPreferences
                  settings={
                    data.routingSettings ?? {
                      remoteCompaction: false,
                    }
                  }
                  disabled={busy || data.enabled}
                  onChange={(settings) =>
                    run(async () => {
                      await call('save_routing_settings', { settings });
                    })
                  }
                />
                <div className="setting-row">
                  <div>
                    <h3>登录电脑时启动</h3>
                    <p className="muted small">默认关闭，由你决定何时启动。</p>
                  </div>
                  <Switch.Root
                    className="switch"
                    checked={data.autostart}
                    disabled={busy || data.enabled}
                    onCheckedChange={(enabled) =>
                      run(async () => {
                        await call('set_autostart', { enabled });
                      })
                    }
                    aria-label="登录电脑时启动"
                  >
                    <Switch.Thumb className="switch-thumb" />
                  </Switch.Root>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>Codex App</h3>
                    <p className="muted small">
                      {data.app.installed ? '已找到应用' : '未找到应用'} ·{' '}
                      {data.app.running ? '正在运行' : '未运行'}
                    </p>
                  </div>
                  <button className="text-button" disabled={busy} onClick={() => run(refresh)}>
                    重新检测
                  </button>
                </div>
              </div>
              <h2 className="settings-section-title">配置与恢复</h2>
              <div className="settings-group">
                <div className="setting-row">
                  <div>
                    <h3>配置位置</h3>
                    <p className="muted small path">{data.configPath}</p>
                  </div>
                </div>
                <div className="setting-row">
                  <div>
                    <h3>恢复开启前的配置</h3>
                    <p className="muted small">关闭开关即可恢复。你的登录和会话保持原样。</p>
                  </div>
                  <button
                    className="text-button"
                    disabled={!data.enabled || busy}
                    onClick={() => toggle(false)}
                  >
                    恢复
                  </button>
                </div>
              </div>
              <div className="settings-note">
                <ShieldCheck size={20} />
                <div>
                  <h3>数据留在你的设备</h3>
                  <p className="muted small">
                    Key 存入系统凭据库。请求由本机发送到所选服务，我们不提供云端中转。
                  </p>
                </div>
              </div>
              <button
                className="secondary full"
                disabled={busy}
                onClick={() =>
                  run(async () => {
                    await call('quit');
                  })
                }
              >
                完全退出 Codex Switch
              </button>
              <p className="form-footnote">
                关闭窗口会保留后台连接。完全退出前请先正常退出 Codex。
              </p>
              <p className="about">Codex Switch {product.version} · 社区独立开源项目</p>
            </section>
          )}
          {view !== 'form' && error && !modal && !data.enabled && (
            <div className="feedback error" role="alert">
              {error}
            </div>
          )}
        </main>
      </div>
      <Modal
        open={data.enabled}
        onClose={() => {}}
        dismissible={false}
        title="Codex Switch 已开启"
        description="请先关闭服务，再修改配置或设置。"
        error={error}
        busy={busy}
      >
        <div className="dialog-actions">
          <button className="primary" disabled={busy} onClick={() => toggle(false)}>
            {busy && <LoaderCircle className="spin" size={16} />}
            {busy ? '正在关闭…' : '关闭服务'}
          </button>
        </div>
      </Modal>
      <Modal
        open={!data.enabled && editingModel !== null}
        onClose={() => setEditingModel(null)}
        title={`编辑 ${editingModel ?? ''} 能力`}
        description="修改仅用于当前配置的这个模型，保存配置后生效。地址和接口默认继承外层配置，能力留空时使用默认值。"
        busy={busy}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!editingModel || busy || data?.enabled || !form.models.includes(editingModel))
              return;
            setForm((current) => ({
              ...current,
              options: {
                ...current.options,
                modelOverrides: {
                  ...current.options?.modelOverrides,
                  [editingModel]: modelDraft,
                },
              },
            }));
            setEditingModel(null);
          }}
        >
          <fieldset className="model-capability-fields" disabled={busy || data?.enabled}>
            <ModelCapabilityFields
              model={editingModel ?? ''}
              spec={modelDraft}
              endpoint={form.endpoint}
              protocol={form.protocol}
              fullUrl={Boolean(form.options?.fullUrl)}
              contextWindow={form.contextWindow}
              disabled={busy || Boolean(data?.enabled)}
              onChange={(change) => setModelDraft((current) => ({ ...current, ...change }))}
            />
          </fieldset>
          <div className="dialog-actions">
            <button
              className="secondary"
              type="button"
              disabled={busy}
              onClick={() => setEditingModel(null)}
            >
              取消
            </button>
            <button className="primary" type="submit" disabled={busy || data?.enabled}>
              确认修改
            </button>
          </div>
        </form>
      </Modal>
      <Modal
        error={error}
        busy={busy}
        open={!data.enabled && modal === 'restart'}
        onClose={() => setModal(null)}
        title="重新打开 Codex"
        description="Codex 正在运行。重启可能中断当前任务，请先完成任务再继续。"
      >
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={() => setModal(null)}>
            稍后
          </button>
          <button
            className="primary"
            disabled={busy}
            onClick={() =>
              run(async () => {
                await call('restart_codex');
                setModal(null);
              })
            }
          >
            正常重启并应用
          </button>
        </div>
      </Modal>
      <Modal
        open={!data.enabled && modal === 'edit-locked'}
        onClose={() => setModal(null)}
        title="请先关闭服务"
        description="Codex Switch 正在运行，请先关闭首页开关，再修改配置。"
      >
        <div className="dialog-actions">
          <button className="primary" onClick={() => setModal(null)}>
            知道了
          </button>
        </div>
      </Modal>
      <Modal
        error={error}
        busy={busy}
        open={!data.enabled && modal === 'delete'}
        onClose={() => setModal(null)}
        title="删除这个配置？"
        description="从列表移除此配置。重新开启后，旧会话的后续请求也将使用当前选中的配置。历史凭据不会自动清理。"
      >
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={() => setModal(null)}>
            取消
          </button>
          <button
            className="primary"
            disabled={busy}
            onClick={() =>
              run(async () => {
                await call('delete_profile', { id: deleteId });
                setModal(null);
              })
            }
          >
            删除配置
          </button>
        </div>
      </Modal>
    </div>
  );
}
