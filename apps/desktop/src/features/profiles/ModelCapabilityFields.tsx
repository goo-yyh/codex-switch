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
        上下文长度
        <input
          aria-label={`${model} 上下文长度`}
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
        {spec.contextNote ?? '内置预设按官方上下文的 80% 设置，预留余量。'}
      </p>
      <label className="field">
        思考档位（逗号分隔）
        <input
          aria-label={`${model} 思考档位（逗号分隔）`}
          className="input"
          placeholder="使用默认能力"
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
        默认思考档位
        <Picker
          label={`${model} 默认思考档位`}
          disabled={disabled}
          value={spec.defaultReasoningLevel ?? 'auto'}
          onChange={(value) =>
            onChange({
              defaultReasoningLevel: value === 'auto' ? undefined : value,
            })
          }
          options={[
            { value: 'auto', label: '自动选择' },
            ...(spec.reasoningLevels ?? [])
              .filter((v) => efforts.includes(v))
              .map((v) => ({ value: v, label: v })),
          ]}
        />
      </label>
      {spec.reasoningNote && <p className="field-hint">{spec.reasoningNote}</p>}
      <label className="field">
        图像输入
        <Picker
          label={`${model} 图像输入`}
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
            { value: 'auto', label: '按模型注册表判断' },
            { value: 'text', label: '仅文本' },
            { value: 'image', label: '文本和图像' },
          ]}
        />
      </label>
      <label className="field">
        并行工具调用
        <Picker
          label={`${model} 并行工具调用`}
          disabled={disabled}
          value={spec.parallelToolCalls === undefined ? 'auto' : String(spec.parallelToolCalls)}
          onChange={(v) =>
            onChange({
              parallelToolCalls: v === 'auto' ? undefined : v === 'true',
            })
          }
          options={[
            { value: 'auto', label: '使用默认能力' },
            { value: 'true', label: '支持' },
            { value: 'false', label: '不支持' },
          ]}
        />
      </label>
    </div>
  );
}
