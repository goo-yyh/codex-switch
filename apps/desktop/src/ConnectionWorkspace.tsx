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
import type { Profile, Snapshot } from './bridge';
import { providerLogos } from '../../../packages/provider-registry/logos';

export function ServiceMark({
  name,
  presetId = 'custom',
  large = false,
}: {
  name: string;
  presetId?: string;
  large?: boolean;
}) {
  return (
    <span
      className={`service-mark ${large ? 'large' : ''}`}
      data-service={presetId}
      aria-hidden="true"
    >
      {providerLogos[presetId] ? (
        <img src={providerLogos[presetId]} alt="" />
      ) : presetId === 'custom' ? (
        '>_'
      ) : (
        name.slice(0, 1)
      )}
    </span>
  );
}

export function ConnectionWorkspace({
  data,
  busy,
  onAdd,
  onSelect,
  onEdit,
  onDelete,
  onToggle,
  onOpen,
  onRecover,
  onSettings,
}: {
  data: Snapshot;
  busy: boolean;
  onAdd: () => void;
  onSelect: (ids: string[]) => void;
  onEdit: (p: Profile) => void;
  onDelete: (p: Profile) => void;
  onToggle: (enabled: boolean) => void;
  onOpen: () => void;
  onRecover: () => void;
  onSettings: () => void;
}) {
  const selected = data.profiles.filter((p) => data.selectedProfiles.includes(p.id));
  const modelCount = selected.reduce((n, p) => n + p.models.length, 0);
  const hasProfiles = data.profiles.length > 0;
  const stateLabel = data.enabled ? (data.routing ? '已开启' : '连接待恢复') : '未开启';
  return (
    <section className="connection-workspace" aria-labelledby="workspace-title">
      <div className="workspace-heading">
        <div>
          <div className="workspace-title">
            <h1 id="workspace-title">我的配置</h1>
            {hasProfiles && <span className="configuration-count">{data.profiles.length}</span>}
          </div>
          <p>选择要在 Codex 中使用的配置，可同时选择多个。</p>
        </div>
        {hasProfiles && (
          <button
            className="secondary add-profile"
            disabled={busy || data.enabled}
            title={data.enabled ? '请先关闭 Codex Switch 再新增配置' : undefined}
            onClick={onAdd}
          >
            <Plus size={16} />
            新增配置
          </button>
        )}
      </div>

      <div className={`configuration-area ${hasProfiles ? '' : 'is-empty'}`}>
        {!hasProfiles ? (
          <div className="configuration-empty">
            <span className="empty-symbol" aria-hidden="true">
              <Layers3 size={25} strokeWidth={1.6} />
            </span>
            <h2>从一个配置开始</h2>
            <p>选择厂商，填入 API Key，添加你常用的模型。</p>
            <button className="primary" disabled={busy || data.enabled} onClick={onAdd}>
              <Plus size={16} />
              新增配置
            </button>
            <span className="empty-caption">一个配置 · 一个 Key · 多个模型</span>
          </div>
        ) : (
          <div className="configuration-grid" role="list" aria-label="已保存的配置">
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
                          {provider} · {p.models.length} 个模型
                        </small>
                      </span>
                      <input
                        type="checkbox"
                        aria-label={`选择 ${p.name}`}
                        aria-describedby={data.enabled ? 'workspace-policy' : undefined}
                        disabled={busy || data.enabled}
                        title={data.enabled ? '请先关闭服务，再选择配置' : undefined}
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
                      {checked ? (data.enabled && data.routing ? '使用中' : '已选择') : '未选择'}
                    </span>
                    <div className="configuration-actions">
                      <button
                        className="icon-button"
                        aria-label={`编辑 ${p.name}`}
                        disabled={busy || data.enabled}
                        title={data.enabled ? '请先关闭 Codex Switch 再编辑配置' : '编辑配置'}
                        onClick={() => onEdit(p)}
                      >
                        <Pencil size={14} />
                      </button>
                      <button
                        className="icon-button danger"
                        aria-label={`删除 ${p.name}`}
                        disabled={busy || data.enabled}
                        title={data.enabled ? '请先关闭 Codex Switch 再删除配置' : '删除配置'}
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

      <section className="connection-controls" aria-label="连接控制">
        <div className="connection-control-row">
          <div className="connection-toggle">
            <Switch.Root
              className="switch"
              checked={data.enabled}
              disabled={busy || (!data.enabled && !selected.length)}
              onCheckedChange={onToggle}
              aria-label="开启 Codex Switch"
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
                  ? `已选 ${selected.length} 个配置 · ${modelCount} 个模型`
                  : '选择配置后即可开启'}
              </p>
            </div>
          </div>
          <button
            className="primary open-codex"
            disabled={busy || !data.enabled}
            title={!data.enabled ? '请先开启 Codex Switch' : undefined}
            onClick={onOpen}
          >
            {busy ? <LoaderCircle className="spin" size={16} /> : <ArrowUpRight size={16} />}
            {data.pendingReload ? '重新打开 Codex' : '打开 Codex'}
          </button>
        </div>
        <div className="connection-control-note" id="workspace-policy">
          <ShieldCheck size={14} aria-hidden="true" />
          <span>
            {data.enabled
              ? '请先关闭服务，再修改配置或设置。'
              : '开启前自动备份，关闭后恢复原配置。'}
          </span>
          <div className="connection-control-actions">
            {data.enabled && !data.routing && (
              <button className="text-button" disabled={busy || data.enabled} onClick={onRecover}>
                恢复连接
              </button>
            )}
            <button
              className="text-button settings-button"
              aria-label="设置"
              disabled={busy || data.enabled}
              onClick={onSettings}
            >
              <Settings size={14} aria-hidden="true" />
              设置
            </button>
          </div>
        </div>
        {data.pendingReload && (
          <p className="connection-reload-note">
            {data.enabled
              ? '路由已更新，完成当前任务后重新打开 Codex 可刷新模型列表。'
              : '已恢复原配置，完成当前任务后重新打开 Codex 生效。'}
          </p>
        )}
      </section>
    </section>
  );
}
