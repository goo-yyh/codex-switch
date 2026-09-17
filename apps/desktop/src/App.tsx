import { useEffect, useRef, useState } from 'react';
import { LoaderCircle, BookOpen } from 'lucide-react';
import { call, isPreview, type Profile } from './api/bridge';
import { Modal } from './components/controls';
import { ConnectionWorkspace } from './features/profiles/ConnectionWorkspace';
import { nextProfileName } from './features/profiles/profileDraft';
import { ProfileEditor } from './features/profiles/ProfileEditor';
import { SettingsPage } from './features/settings/SettingsPage';
import { useAppController } from './hooks/useAppController';
import { documentationUrl } from './product';
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
  const controller = useAppController();
  const { data, busy, error, setError, setNotice, run } = controller;
  const mainRef = useRef<HTMLElement>(null);
  const [view, setView] = useState<'home' | 'form' | 'settings'>('home');
  const [form, setForm] = useState<Profile>(newConnection);
  const [modal, setModal] = useState<'restart' | 'delete' | null>(null);
  const [deleteId, setDeleteId] = useState('');
  useEffect(() => {
    if (data?.enabled) setModal(null);
  }, [data?.enabled]);
  useEffect(() => {
    if (mainRef.current) mainRef.current.scrollTop = 0;
  }, [view]);
  function add() {
    if (data?.enabled) {
      setError('');
      return;
    }
    const p = data?.presets[0];
    setForm({
      ...newConnection(),
      presetId: p?.id || 'custom',
      name: nextProfileName(data?.profiles ?? [], p?.name || 'coding_plan', p?.id || 'custom'),
      endpoint: p?.endpoint || '',
      models: p ? [p.model] : [],
      protocol: p?.protocol || 'responses',
      contextWindow: p?.contextWindow || 32768,
      options: structuredClone(p?.options ?? {}),
    });

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
                  return;
                }
                setForm(c);

                setError('');
                setNotice('');
                setView('form');
              }}
              onDelete={(c) => {
                if (data.enabled) {
                  setError('');
                  return;
                }
                setError('');
                setDeleteId(c.id);
                setModal('delete');
              }}
              onToggle={toggle}
              onSettings={() => {
                setView('settings');

                setError('');
                setNotice('');
              }}
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
            <ProfileEditor
              data={data}
              controller={controller}
              initialProfile={form}
              onBack={() => setView('home')}
            />
          )}
          {view === 'settings' && (
            <SettingsPage data={data} controller={controller} onBack={() => setView('home')} />
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
