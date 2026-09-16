import { useEffect, useState } from 'react';
import * as Switch from '@radix-ui/react-switch';
import { Picker } from './components';
import type { Profile, RoutingSettings } from './bridge';
export function CompatibilityFields({
  form,
  onChange,
  disabled,
}: {
  form: Profile;
  onChange: (p: Profile) => void;
  disabled: boolean;
}) {
  const options = form.options ?? {};
  const hasChat = form.models.some(
    (model) => (options.modelOverrides?.[model]?.protocol ?? form.protocol) === 'chat',
  );
  if (!hasChat) return null;
  return (
    <div className="compatibility-fields">
      {hasChat && (
        <details>
          <summary>Chat 思考参数</summary>
          <p className="field-hint">思考参数以当前服务的接口文档为准。</p>
          <label className="field">
            参数方案
            <Picker
              label="Chat 思考参数方案"
              disabled={disabled}
              value={options.chatReasoning?.thinkingParam ?? 'auto'}
              options={[
                { value: 'auto', label: '不注入专用参数' },
                { value: 'thinking', label: 'thinking（DeepSeek / Kimi / GLM）' },
                { value: 'enable_thinking', label: 'enable_thinking（千问）' },
                { value: 'reasoning_split', label: 'reasoning_split（MiniMax）' },
              ]}
              onChange={(v) =>
                onChange({
                  ...form,
                  options: {
                    ...options,
                    chatReasoning:
                      v === 'auto'
                        ? null
                        : {
                            supportsThinking: true,
                            supportsEffort: false,
                            thinkingParam: v,
                            effortParam: 'none',
                            outputFormat: 'reasoning_content',
                          },
                  },
                })
              }
            />
          </label>
          {options.chatReasoning && (
            <label className="field">
              推理强度参数
              <Picker
                label="推理强度参数"
                disabled={disabled}
                value={
                  options.chatReasoning.supportsEffort
                    ? (options.chatReasoning.effortValueMode ?? 'passthrough')
                    : 'none'
                }
                options={[
                  { value: 'none', label: '不发送 effort' },
                  { value: 'passthrough', label: 'reasoning_effort 原值' },
                  { value: 'deepseek', label: 'DeepSeek 档位映射' },
                ]}
                onChange={(v) =>
                  onChange({
                    ...form,
                    options: {
                      ...options,
                      chatReasoning: {
                        ...options.chatReasoning,
                        supportsEffort: v !== 'none',
                        effortParam: v === 'none' ? 'none' : 'reasoning_effort',
                        effortValueMode: v === 'none' ? undefined : v,
                      },
                    },
                  })
                }
              />
            </label>
          )}
        </details>
      )}
    </div>
  );
}
export function RoutingPreferences({
  profiles,
  settings,
  disabled,
  onSave,
}: {
  profiles: Profile[];
  settings: RoutingSettings;
  disabled: boolean;
  onSave: (s: RoutingSettings) => void;
}) {
  const [draft, setDraft] = useState(settings);
  useEffect(
    () => setDraft(settings),
    [
      settings.remoteCompaction,
      settings.failoverEnabled,
      JSON.stringify(settings.fallbackProfiles),
    ],
  );
  return (
    <section className="settings-group routing-preferences">
      <h2>压缩与备用队列</h2>
      <div className="setting-row">
        <div>
          <h3>远程上下文压缩</h3>
          <p className="muted small">
            默认关闭：Codex 通过普通模型请求生成摘要并整理上下文。开启后按 Codex
            客户端的远程压缩机制请求上游，需要服务支持对应协议；不支持时可能失败，不保证自动回退。
          </p>
        </div>
        <Switch.Root
          className="switch"
          aria-label="远程上下文压缩"
          aria-describedby="compaction-scope"
          checked={draft.remoteCompaction}
          disabled={disabled}
          onCheckedChange={(v) => setDraft({ ...draft, remoteCompaction: v })}
        >
          <Switch.Thumb className="switch-thumb" />
        </Switch.Root>
      </div>
      <p id="compaction-scope" className="muted small">
        适用于所有已选配置。保存后下次开启 Codex Switch 时生效；已运行的 Codex
        可能需要重新加载配置。
      </p>
      <div className="setting-row">
        <div>
          <h3>启用备用队列</h3>
          <p className="muted small">
            开启后，请求仅按下列顺序使用各配置的首个模型及其密钥。连接、鉴权、限流或服务错误时尝试下一项，流式响应开始后不切换。
          </p>
        </div>
        <Switch.Root
          className="switch"
          aria-label="启用备用队列"
          checked={draft.failoverEnabled}
          disabled={disabled}
          onCheckedChange={(v) => setDraft({ ...draft, failoverEnabled: v })}
        >
          <Switch.Thumb className="switch-thumb" />
        </Switch.Root>
      </div>
      <ol>
        {draft.fallbackProfiles.map((id, i) => (
          <li key={id}>
            <span>{profiles.find((p) => p.id === id)?.name ?? '已删除的配置'}</span>{' '}
            <button
              className="text-button small"
              disabled={disabled || i === 0}
              onClick={() => {
                const queue = [...draft.fallbackProfiles];
                [queue[i - 1], queue[i]] = [queue[i], queue[i - 1]];
                setDraft({ ...draft, fallbackProfiles: queue });
              }}
            >
              上移
            </button>{' '}
            <button
              className="text-button small"
              disabled={disabled}
              onClick={() =>
                setDraft({
                  ...draft,
                  fallbackProfiles: draft.fallbackProfiles.filter((p) => p !== id),
                })
              }
            >
              移除
            </button>
          </li>
        ))}
      </ol>
      {profiles
        .filter((p) => !draft.fallbackProfiles.includes(p.id))
        .map((p) => (
          <button
            className="text-button small"
            key={p.id}
            disabled={disabled || draft.fallbackProfiles.length >= 8}
            onClick={() =>
              setDraft({ ...draft, fallbackProfiles: [...draft.fallbackProfiles, p.id] })
            }
          >
            添加 {p.name}
          </button>
        ))}
      <p className="field-hint">
        如需兼容 Chat，请单独保存一个 Chat
        配置再加入队列；不会自动修改原配置的协议。预设地址只是初始值。
      </p>
      <button
        className="primary"
        disabled={disabled || (draft.failoverEnabled && !draft.fallbackProfiles.length)}
        onClick={() => onSave(draft)}
      >
        保存路由设置
      </button>
      {disabled && <p className="field-hint">关闭 Codex Switch 后可修改，下次开启生效。</p>}
    </section>
  );
}
