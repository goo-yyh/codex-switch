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
export function CompactionPreferences({
  settings,
  disabled,
  onChange,
}: {
  settings: RoutingSettings;
  disabled: boolean;
  onChange: (s: RoutingSettings) => void;
}) {
  return (
    <div className="setting-row" title={disabled ? '请先关闭 Codex Switch 后再修改' : undefined}>
      <div>
        <h3>远程上下文压缩</h3>
        <p id="compaction-description" className="muted small">
          由服务端压缩上下文，需服务支持，建议不开启。
        </p>
      </div>
      <Switch.Root
        className="switch"
        aria-label="远程上下文压缩"
        aria-describedby="compaction-description"
        checked={settings.remoteCompaction}
        disabled={disabled}
        onCheckedChange={(remoteCompaction) => onChange({ remoteCompaction })}
      >
        <Switch.Thumb className="switch-thumb" />
      </Switch.Root>
    </div>
  );
}
