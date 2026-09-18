import { LocaleContext, useI18n } from './i18n';
import { useEffect, useRef, useState } from 'react';
import * as Switch from '@radix-ui/react-switch';
import { LoaderCircle, BookOpen, Languages } from 'lucide-react';
import { call, isPreview, type Profile } from './api/bridge';
import { Modal } from './components/controls';
import { ConnectionWorkspace } from './features/profiles/ConnectionWorkspace';
import { nextProfileName } from './features/profiles/profileDraft';
import { ProfileEditor } from './features/profiles/ProfileEditor';
import { SettingsPage } from './features/settings/SettingsPage';
import { UpdateButton } from './features/updates/UpdateButton';
import { useAppController, type AppController } from './hooks/useAppController';
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
  const locale = controller.data?.locale ?? 'zh-CN';
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return (
    <LocaleContext.Provider value={locale}>
      <AppContent controller={controller} />
    </LocaleContext.Provider>
  );
}
function AppContent({ controller }: { controller: AppController }) {
  const { t, text, locale, message } = useI18n();
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
      name: nextProfileName(
        data?.profiles ?? [],
        text(p?.name || 'coding_plan'),
        p?.id || 'custom',
      ),
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
        <p>{message(error) || t('正在准备 Codex Switch…')}</p>
      </div>
    );
  return (
    <div className="app-shell">
      <header className="app-header" inert={data.enabled}>
        <button
          className="brand"
          disabled={busy}
          aria-label={t('回到首页')}
          onClick={() => {
            setView('home');

            setError('');
            setNotice('');
          }}
        >
          <img className="brand-symbol" src={brandIcon} alt="" />
          <span>Codex Switch</span>
        </button>
        <nav className="header-actions" aria-label={t('应用导航')}>
          <UpdateButton disabled={busy || data.enabled} run={run} />
          <button
            className="header-link"
            disabled={busy || data.enabled}
            aria-label={t(locale === 'zh-CN' ? '切换到英文' : '切换到中文')}
            onClick={() =>
              run(async () => {
                await call('set_locale', { locale: locale === 'zh-CN' ? 'en' : 'zh-CN' });
              })
            }
          >
            <Languages size={16} aria-hidden="true" />
            <span>{locale === 'zh-CN' ? 'English' : '简体中文'}</span>
          </button>
          <button
            className="header-link"
            disabled={busy}
            aria-label={t('使用文档')}
            onClick={() =>
              run(async () => {
                await call('open_link', { url: documentationUrl(locale) });
              })
            }
          >
            <BookOpen size={16} />
            <span>{t('文档')}</span>
          </button>
        </nav>
      </header>
      <div className="app-content" inert={data.enabled}>
        {isPreview && (
          <div className="preview-bar">
            {t('交互预览 · 数据仅保存在本页，不会修改配置或调用服务')}
          </div>
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
              {message(error)}
            </div>
          )}
        </main>
      </div>
      <Modal
        open={data.enabled}
        onClose={() => {}}
        dismissible={false}
        title={t('Codex Switch 已开启')}
        description={t('请先关闭服务，再修改配置或设置。')}
        error={message(error)}
        busy={busy}
      >
        <div className="dialog-actions service-switch">
          <span className="muted small" aria-live="polite">
            {busy ? t('正在关闭…') : t('已开启')}
          </span>
          <Switch.Root
            className="switch"
            checked={data.enabled}
            disabled={busy}
            onCheckedChange={toggle}
            aria-label={t('Codex Switch 服务')}
          >
            <Switch.Thumb className="switch-thumb" />
          </Switch.Root>
        </div>
      </Modal>
      <Modal
        error={message(error)}
        busy={busy}
        open={!data.enabled && modal === 'restart'}
        onClose={() => setModal(null)}
        title={t('重新打开 Codex')}
        description={t('Codex 正在运行。重启可能中断当前任务，请先完成任务再继续。')}
      >
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={() => setModal(null)}>
            {t('稍后')}
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
            {t('正常重启并应用')}
          </button>
        </div>
      </Modal>
      <Modal
        error={message(error)}
        busy={busy}
        open={!data.enabled && modal === 'delete'}
        onClose={() => setModal(null)}
        title={t('删除这个配置？')}
        description={t(
          '从列表移除此配置。重新开启后，旧会话的后续请求也将使用当前选中的配置。历史凭据不会自动清理。',
        )}
      >
        <div className="dialog-actions">
          <button className="secondary" disabled={busy} onClick={() => setModal(null)}>
            {t('取消')}
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
            {t('删除配置')}
          </button>
        </div>
      </Modal>
    </div>
  );
}
