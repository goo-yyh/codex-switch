import { useEffect, useState } from 'react';
import { ArrowLeft, ArrowUpRight, Check, Eye, EyeOff, LoaderCircle } from 'lucide-react';
import { call, type Snapshot, type Profile, type ModelOptions } from '../../api/bridge';
import type { AppController } from '../../hooks/useAppController';
import { Picker, Modal } from '../../components/controls';
import { ServiceMark } from '../../components/ServiceMark';
import { CompatibilityFields } from './CompatibilityFields';
import { ModelSelection } from './ModelSelection';
import { RequestFields } from './RequestFields';
import { ModelCapabilityFields } from './ModelCapabilityFields';
import { canReuseCredential, nextProfileName } from './profileDraft';
import { withReasoningDefaults } from './modelDefaults';

export function ProfileEditor({
  data,
  controller,
  initialProfile,
  onBack,
}: {
  data: Snapshot;
  controller: AppController;
  initialProfile: Profile;
  onBack: () => void;
}) {
  const { busy, busyRef, phase, setPhase, error, setError, notice, setNotice, run } = controller;
  const [form, setForm] = useState<Profile>(initialProfile);
  const [editingModel, setEditingModel] = useState<string | null>(null);
  const [modelDraft, setModelDraft] = useState<ModelOptions>({});
  const [key, setKey] = useState('');
  const models = form.models;
  const [nameEdited, setNameEdited] = useState(Boolean(initialProfile.id));
  const [showKey, setShowKey] = useState(false);
  const [validationId, setValidationId] = useState<string | null>(null);
  // A tray activation locks this page too; never retain a visible credential draft.
  useEffect(() => {
    if (data.enabled) {
      setEditingModel(null);
      setKey('');
      setShowKey(false);
    }
  }, [data.enabled]);
  function usePreset(id: string) {
    if (form.id || id === form.presetId) return;
    setKey('');
    setShowKey(false);
    const p = data?.presets.find((p) => p.id === id);
    setForm((f) => ({
      ...f,
      presetId: id,
      name: nameEdited
        ? f.name
        : nextProfileName(data.profiles, p?.name || 'coding_plan', id, form.id),
      endpoint: p?.endpoint || '',
      models: p ? [p.model] : [],
      protocol: p?.protocol || 'responses',
      contextWindow: p?.contextWindow || 32768,
      options: structuredClone(p?.options ?? {}),
    }));
  }
  async function save() {
    if (busyRef.current) return;
    if (data?.enabled) {
      setError('');
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
      onBack();
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
      const receipt = await call<import('../../api/bridge').Receipt>('probe_endpoint', {
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
  const canReuseKey = canReuseCredential(form, original);
  const nameConflict = Boolean(
    data?.profiles.some(
      (p) => p.id !== form.id && p.name.toLowerCase() === form.name.trim().toLowerCase(),
    ),
  );
  const phaseLabel = {
    idle: '保存配置',
    working: '处理中…',
    testing: '保存配置',
  }[phase];
  return (
    <>
      <section className="form-page">
        <div className="form-titlebar">
          <button
            className="text-button back"
            disabled={busy}
            onClick={() => {
              onBack();
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
                    <ServiceMark name={preset?.name || 'coding plan'} presetId={form.presetId} />
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
                      placeholder={canReuseKey ? '已保存，留空保留原密钥' : '粘贴当前服务的密钥'}
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
            {error && (
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
                disabled={busy || data.enabled || nameConflict || !models.length || !form.endpoint}
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
      </section>{' '}
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
    </>
  );
}
