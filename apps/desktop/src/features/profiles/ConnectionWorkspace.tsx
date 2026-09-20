import { useI18n } from '../../i18n';
import * as Switch from '@radix-ui/react-switch';
import {
  ArrowUpRight,
  Layers3,
  LoaderCircle,
  Pencil,
  Plus,
  Settings,
  ShieldCheck,
  Trash2,
} from 'lucide-react';
import type { Profile, Snapshot } from '../../api/bridge';
import { ServiceMark } from '../../components/ServiceMark';

export function ConnectionWorkspace({
  data,
  busy,
  error,
  onAdd,
  onSelect,
  onEdit,
  onDelete,
  onToggle,
  onOpen,
  onSettings,
}: {
  data: Snapshot;
  busy: boolean;
  error?: string;
  onAdd: () => void;
  onSelect: (ids: string[]) => void;
  onEdit: (p: Profile) => void;
  onDelete: (p: Profile) => void;
  onToggle: (enabled: boolean) => void;
  onOpen: () => void;
  onSettings: () => void;
}) {
  const { t, text } = useI18n();
  const selected = data.profiles.filter((p) => data.selectedProfiles.includes(p.id));
  const modelCount = selected.reduce((n, p) => n + p.models.length, 0);
  const hasProfiles = data.profiles.length > 0;
  const stateLabel = data.enabled ? (data.routing ? t('已开启') : t('连接待恢复')) : t('未开启');
  return (
    <section className="connection-workspace" aria-labelledby="workspace-title">
      <div className="workspace-heading" inert={data.enabled}>
        <div>
          <div className="workspace-title">
            <h1 id="workspace-title">{t('我的配置')}</h1>
            {hasProfiles && <span className="configuration-count">{data.profiles.length}</span>}
          </div>
          <p>{t('选择要在 Codex 中使用的配置，可同时选择多个。')}</p>
        </div>
        {hasProfiles && (
          <button
            className="secondary add-profile"
            disabled={busy || data.enabled}
            title={data.enabled ? t('请先关闭 Codex Switch 再新增配置') : undefined}
            onClick={onAdd}
          >
            <Plus size={16} />
            {t('新增配置')}
          </button>
        )}
      </div>

      <div className={`configuration-area ${hasProfiles ? '' : 'is-empty'}`} inert={data.enabled}>
        {!hasProfiles ? (
          <div className="configuration-empty">
            <span className="empty-symbol" aria-hidden="true">
              <Layers3 size={25} strokeWidth={1.6} />
            </span>
            <h2>{t('从一个配置开始')}</h2>
            <p>{t('选择厂商，填入 API Key，添加你常用的模型。')}</p>
            <button className="primary" disabled={busy || data.enabled} onClick={onAdd}>
              <Plus size={16} />
              {t('新增配置')}
            </button>
            <span className="empty-caption">{t('一个配置 · 一个 Key · 多个模型')}</span>
          </div>
        ) : (
          <div className="configuration-grid" role="list" aria-label={t('已保存的配置')}>
            {data.profiles.map((p) => {
              const checked = data.selectedProfiles.includes(p.id);
              const provider = data.presets.find((v) => v.id === p.presetId)?.name || 'coding plan';
              return (
                <article
                  className={`configuration-card ${checked ? 'is-selected' : ''}`}
                  key={p.id}
                  role="listitem"
                >
                  <label className="configuration-choice">
                    <span className="configuration-card-heading">
                      <ServiceMark name={p.name} presetId={p.presetId} />
                      <span className="configuration-name">
                        <strong title={p.name}>{p.name}</strong>
                        <small>
                          {text(provider)} · {t('{count} 个模型', { count: p.models.length })}
                        </small>
                      </span>
                      <input
                        type="checkbox"
                        aria-label={t('选择 {value0}', { value0: p.name })}
                        aria-describedby={data.enabled ? 'workspace-policy' : undefined}
                        disabled={busy || data.enabled}
                        title={data.enabled ? t('请先关闭服务，再选择配置') : undefined}
                        checked={checked}
                        onChange={() =>
                          onSelect(
                            checked
                              ? data.selectedProfiles.filter((id) => id !== p.id)
                              : [...data.selectedProfiles, p.id],
                          )
                        }
                      />
                    </span>
                    <span className="configuration-models" title={p.models.join('、')}>
                      {p.models.slice(0, 2).map((model) => (
                        <span className="configuration-model" key={model}>
                          {model}
                        </span>
                      ))}
                      {p.models.length > 2 && (
                        <span className="configuration-overflow">+{p.models.length - 2}</span>
                      )}
                    </span>
                  </label>
                  <div className="configuration-card-footer">
                    <span className={`configuration-state ${checked ? 'is-selected' : ''}`}>
                      <span aria-hidden="true" />
                      {checked
                        ? data.enabled && data.routing
                          ? t('使用中')
                          : t('已选择')
                        : t('未选择')}
                    </span>
                    <div className="configuration-actions">
                      <button
                        className="icon-button"
                        aria-label={t('编辑 {value0}', { value0: p.name })}
                        disabled={busy || data.enabled}
                        title={data.enabled ? t('请先关闭 Codex Switch 再编辑配置') : t('编辑配置')}
                        onClick={() => onEdit(p)}
                      >
                        <Pencil size={14} />
                      </button>
                      <button
                        className="icon-button danger"
                        aria-label={t('删除 {value0}', { value0: p.name })}
                        disabled={busy || data.enabled}
                        title={data.enabled ? t('请先关闭 Codex Switch 再删除配置') : t('删除配置')}
                        onClick={() => onDelete(p)}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </div>

      <section
        className={`connection-controls ${data.enabled ? 'is-service-active' : ''}`}
        aria-label={t('连接控制')}
      >
        <div className="connection-control-row">
          <div className="connection-toggle">
            <Switch.Root
              className="switch"
              checked={data.enabled}
              disabled={busy || (!data.enabled && !selected.length)}
              onCheckedChange={onToggle}
              aria-label={t('开启 Codex Switch')}
              aria-describedby="connection-selection"
            >
              <Switch.Thumb className="switch-thumb" />
            </Switch.Root>
            <div>
              <span className="connection-state-label">
                Codex Switch <strong data-enabled={data.enabled}>{stateLabel}</strong>
              </span>
              <p id="connection-selection">
                {selected.length
                  ? t('已选 {value0} 个配置 · {value1} 个模型', {
                      value0: selected.length,
                      value1: modelCount,
                    })
                  : t('选择配置后即可开启')}
              </p>
            </div>
          </div>
          <button
            className="primary open-codex"
            disabled={busy || !data.enabled}
            title={!data.enabled ? t('请先开启 Codex Switch') : undefined}
            onClick={onOpen}
          >
            {busy ? <LoaderCircle className="spin" size={16} /> : <ArrowUpRight size={16} />}
            {data.pendingReload ? t('重新打开 Codex') : t('打开 Codex')}
          </button>
        </div>
        <div className="connection-control-note" id="workspace-policy">
          <ShieldCheck size={14} aria-hidden="true" />
          <span>
            {data.enabled
              ? t('请先关闭服务，再修改配置或设置。')
              : t('开启前自动备份，关闭后恢复原配置。')}
          </span>
          <div className="connection-control-actions">
            <button
              className="text-button settings-button"
              aria-label={t('设置')}
              disabled={busy || data.enabled}
              onClick={onSettings}
            >
              <Settings size={14} aria-hidden="true" />
              {t('设置')}
            </button>
          </div>
        </div>
        {data.enabled && error && (
          <div className="feedback error" role="alert">
            {error}
          </div>
        )}
        {data.pendingReload && (
          <p className="connection-reload-note">
            {data.enabled
              ? t('路由已更新，完成当前任务后重新打开 Codex 可刷新模型列表。')
              : t('已恢复原配置，完成当前任务后重新打开 Codex 生效。')}
          </p>
        )}
      </section>
    </section>
  );
}
