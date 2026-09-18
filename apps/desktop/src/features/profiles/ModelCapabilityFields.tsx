import { useI18n } from '../../i18n';
import { Picker } from '../../components/controls';
import { RequestFields } from './RequestFields';
import type { ModelOptions, Protocol } from '../../api/bridge';
const efforts = ['none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra'];

export function ModelCapabilityFields({
  model,
  spec,
  contextWindow,
  endpoint,
  protocol,
  fullUrl,
  disabled,
  onChange,
}: {
  model: string;
  spec: ModelOptions;
  contextWindow: number;
  endpoint: string;
  protocol: Protocol;
  fullUrl: boolean;
  disabled: boolean;
  onChange: (change: Partial<ModelOptions>) => void;
}) {
  const { t, text } = useI18n();
  return (
    <div className="model-capability">
      <RequestFields
        model={model}
        endpoint={spec.endpoint ?? ''}
        protocol={spec.protocol}
        fullUrl={spec.fullUrl ?? fullUrl}
        inherited={{ endpoint, protocol, fullUrl }}
        disabled={disabled}
        onChange={onChange}
      />
      <label className="field">
        {t('上下文长度')}
        <input
          aria-label={t('{value0} 上下文长度', { value0: model })}
          type="number"
          className="input"
          min={4096}
          max={2000000}
          placeholder={String(contextWindow)}
          value={spec.contextWindow ?? ''}
          onChange={(e) =>
            onChange({
              contextWindow: e.target.value ? Number(e.target.value) : undefined,
            })
          }
        />
      </label>
      <p className="field-hint">
        {spec.contextNote
          ? text(spec.contextNote)
          : t('内置预设按官方上下文的 80% 设置，预留余量。')}
      </p>
      <label className="field">
        {t('思考档位（逗号分隔）')}
        <input
          aria-label={t('{value0} 思考档位（逗号分隔）', { value0: model })}
          className="input"
          placeholder={t('使用默认能力')}
          value={spec.reasoningLevels?.join(',') ?? ''}
          onChange={(e) => {
            const values = e.target.value.split(',').map((v) => v.trim());
            onChange({
              reasoningLevels: e.target.value ? values : undefined,
              defaultReasoningLevel: undefined,
            });
          }}
        />
      </label>
      <label className="field">
        {t('默认思考档位')}
        <Picker
          label={t('{value0} 默认思考档位', { value0: model })}
          disabled={disabled}
          value={spec.defaultReasoningLevel ?? 'auto'}
          onChange={(value) =>
            onChange({
              defaultReasoningLevel: value === 'auto' ? undefined : value,
            })
          }
          options={[
            { value: 'auto', label: t('自动选择') },
            ...(spec.reasoningLevels ?? [])
              .filter((v) => efforts.includes(v))
              .map((v) => ({ value: v, label: v })),
          ]}
        />
      </label>
      {spec.reasoningNote && <p className="field-hint">{text(spec.reasoningNote)}</p>}
      <label className="field">
        {t('图像输入')}
        <Picker
          label={t('{value0} 图像输入', { value0: model })}
          disabled={disabled}
          value={
            spec.inputModalities
              ? spec.inputModalities.includes('image')
                ? 'image'
                : 'text'
              : 'auto'
          }
          onChange={(v) =>
            onChange({
              inputModalities:
                v === 'auto' ? undefined : v === 'image' ? ['text', 'image'] : ['text'],
            })
          }
          options={[
            { value: 'auto', label: t('按模型注册表判断') },
            { value: 'text', label: t('仅文本') },
            { value: 'image', label: t('文本和图像') },
          ]}
        />
      </label>
      <label className="field">
        {t('并行工具调用')}
        <Picker
          label={t('{value0} 并行工具调用', { value0: model })}
          disabled={disabled}
          value={spec.parallelToolCalls === undefined ? 'auto' : String(spec.parallelToolCalls)}
          onChange={(v) =>
            onChange({
              parallelToolCalls: v === 'auto' ? undefined : v === 'true',
            })
          }
          options={[
            { value: 'auto', label: t('使用默认能力') },
            { value: 'true', label: t('支持') },
            { value: 'false', label: t('不支持') },
          ]}
        />
      </label>
    </div>
  );
}
