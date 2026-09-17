import { useId } from 'react';
import * as Switch from '@radix-ui/react-switch';
import { Picker } from '../../components/controls';
import type { ModelOptions, Protocol } from '../../api/bridge';

export function RequestFields({
  model,
  endpoint,
  protocol,
  fullUrl,
  inherited,
  disabled,
  onChange,
}: {
  model?: string;
  endpoint: string;
  protocol?: Protocol;
  fullUrl: boolean;
  inherited?: { endpoint: string; protocol: Protocol; fullUrl: boolean };
  disabled: boolean;
  onChange: (change: Partial<ModelOptions>) => void;
}) {
  const id = useId();
  const label = (text: string) => (model ? `${model} ${text}` : text);
  return (
    <div className="request-fields">
      <div className="request-address-row">
        <div className="field">
          <div className="request-address-label">
            <label htmlFor={`${id}-endpoint`}>API 地址</label>
            <div className="request-url-mode">
              <label htmlFor={`${id}-full-url`}>完整 URL</label>
              <Switch.Root
                id={`${id}-full-url`}
                className="switch switch-compact"
                aria-label={label('完整 URL')}
                checked={fullUrl}
                disabled={disabled}
                onCheckedChange={(value) => onChange({ fullUrl: value })}
              >
                <Switch.Thumb className="switch-thumb" />
              </Switch.Root>
            </div>
          </div>
          <input
            id={`${id}-endpoint`}
            className="input"
            type="url"
            aria-label={label('API 地址')}
            required={!inherited}
            disabled={disabled}
            value={endpoint}
            placeholder={
              inherited?.endpoint ??
              (fullUrl ? 'https://api.example.com/v1/responses' : 'https://api.example.com/v1')
            }
            onChange={(event) =>
              onChange({
                endpoint: inherited && !event.target.value.trim() ? undefined : event.target.value,
              })
            }
          />
        </div>
        <label className="field">
          接口格式
          <Picker
            label={label('接口格式')}
            disabled={disabled}
            value={protocol ?? 'inherit'}
            onChange={(value) =>
              onChange({ protocol: value === 'inherit' ? undefined : (value as Protocol) })
            }
            options={[
              ...(inherited
                ? [
                    {
                      value: 'inherit',
                      label: `继承（${inherited.protocol === 'responses' ? 'Responses' : 'Chat Completions'}）`,
                    },
                  ]
                : []),
              { value: 'responses', label: 'Responses' },
              { value: 'chat', label: 'Chat Completions' },
            ]}
          />
        </label>
      </div>
      {inherited && (
        <button
          type="button"
          className="text-button small request-inherit"
          disabled={disabled}
          onClick={() => onChange({ endpoint: undefined, protocol: undefined, fullUrl: undefined })}
        >
          继承外层设置
        </button>
      )}
      {(fullUrl || inherited) && (
        <p className="field-hint">
          {fullUrl ? '直接请求填写的完整地址，不追加后缀。' : '地址留空时继承外层 API 地址。'}
        </p>
      )}
    </div>
  );
}
