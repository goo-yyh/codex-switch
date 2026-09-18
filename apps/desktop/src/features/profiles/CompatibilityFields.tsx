import { useI18n } from '../../i18n';
import { Picker } from '../../components/controls';
import type { Profile } from '../../api/bridge';
export function CompatibilityFields({
  form,
  onChange,
  disabled,
}: {
  form: Profile;
  onChange: (p: Profile) => void;
  disabled: boolean;
}) {
  const { t } = useI18n();
  const options = form.options ?? {};
  const hasChat = form.models.some(
    (model) => (options.modelOverrides?.[model]?.protocol ?? form.protocol) === 'chat',
  );
  if (!hasChat) return null;
  return (
    <div className="compatibility-fields">
      <details>
        <summary>{t('Chat 思考参数')}</summary>
        <p className="field-hint">{t('思考参数以当前服务的接口文档为准。')}</p>
        <label className="field">
          {t('参数方案')}
          <Picker
            label={t('Chat 思考参数方案')}
            disabled={disabled}
            value={options.chatReasoning?.thinkingParam ?? 'auto'}
            options={[
              { value: 'auto', label: t('不注入专用参数') },
              { value: 'thinking', label: 'thinking（DeepSeek / Kimi / GLM）' },
              { value: 'enable_thinking', label: t('enable_thinking（千问）') },
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
            {t('推理强度参数')}
            <Picker
              label={t('推理强度参数')}
              disabled={disabled}
              value={
                options.chatReasoning.supportsEffort
                  ? (options.chatReasoning.effortValueMode ?? 'passthrough')
                  : 'none'
              }
              options={[
                { value: 'none', label: t('不发送 effort') },
                { value: 'passthrough', label: t('reasoning_effort 原值') },
                { value: 'deepseek', label: t('DeepSeek 档位映射') },
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
    </div>
  );
}
