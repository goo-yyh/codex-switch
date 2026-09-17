import { useId, useState } from 'react';
import { Picker } from '../../components/controls';
import catalog from '../../../../../packages/provider-registry/models.json';

const CUSTOM = '__custom_model__';

export function ModelSelection({
  provider,
  availableModels,
  models,
  onChange,
  multiple,
  disabled,
  onEditModel,
}: {
  provider: string;
  availableModels?: string[];
  models: string[];
  onChange: (models: string[]) => void;
  multiple: boolean;
  disabled: boolean;
  onEditModel?: (model: string) => void;
}) {
  const statusId = useId();
  const available: string[] =
    availableModels ?? (catalog as Record<string, string[]>)[provider] ?? [];
  const [customModels, setCustomModels] = useState(() =>
    models.filter((m) => !available.includes(m)),
  );
  const [custom, setCustom] = useState(false);
  const [draft, setDraft] = useState('');
  const [error, setError] = useState('');
  const current = models[0] ?? '';
  const customMode = custom || !available.includes(current);

  if (!multiple)
    return (
      <div className="field">
        <label htmlFor="model">模型</label>
        {available.length > 0 && (
          <Picker
            id="model"
            label="模型"
            disabled={disabled}
            value={customMode ? CUSTOM : current}
            options={[
              ...available.map((value) => ({
                value,
                label: value,
              })),
              { value: CUSTOM, label: '自定义模型…' },
            ]}
            onChange={(value) => {
              setCustom(value === CUSTOM);
              if (value !== CUSTOM) onChange([value]);
            }}
          />
        )}
        {(available.length === 0 || customMode) && (
          <input
            id={available.length ? 'custom-model' : 'model'}
            aria-label={available.length ? '自定义模型 ID' : '模型'}
            className="input"
            value={current}
            required
            maxLength={200}
            disabled={disabled}
            onChange={(e) => onChange([e.target.value])}
            placeholder="输入服务商提供的模型 ID"
          />
        )}
      </div>
    );

  function addCustom() {
    const value = draft.trim();
    if (disabled || !value) return;
    if ([...available, ...customModels, ...models].includes(value)) {
      setError('该模型 ID 已存在，请在列表中勾选。');
      return;
    }
    if (models.length >= 20) {
      setError('最多选择 20 个模型。');
      return;
    }
    setCustomModels((items) => [value, ...items]);
    onChange([...models, value]);
    setDraft('');
    setError('');
  }

  // Display order is independent of selection order, whose first item is the default.
  const candidates = [...new Set([...customModels, ...available, ...models])];
  const choices = [
    ...candidates.filter((model) => models.includes(model)),
    ...candidates.filter((model) => !models.includes(model)),
  ];
  return (
    <div className="field model-selection">
      <div className="form-card-heading">
        <h2 id="models-label">模型选择</h2>
        <span className="muted small">已选 {models.length}</span>
      </div>
      {choices.length > 0 && (
        <div className="model-options" role="group" aria-labelledby="models-label">
          {choices.map((model) => {
            const selected = models.includes(model);
            const isDefault = models[0] === model;
            const isCustom = !available.includes(model);
            return (
              <div className="model-option" key={model}>
                <label className="model-choice" title={model}>
                  <input
                    type="checkbox"
                    checked={selected}
                    aria-label={model}
                    aria-describedby={isDefault ? statusId : undefined}
                    disabled={disabled || (models.length >= 20 && !selected)}
                    onChange={(e) => {
                      setError('');
                      onChange(
                        e.target.checked ? [...models, model] : models.filter((m) => m !== model),
                      );
                    }}
                  />
                  <span>{model}</span>
                  {isDefault && (
                    <span className="model-default" id={statusId}>
                      默认
                    </span>
                  )}
                </label>
                <span className="model-status">
                  {isCustom && (
                    <button
                      className="text-button model-delete"
                      type="button"
                      disabled={disabled}
                      aria-label={`删除模型 ${model}`}
                      title="删除模型"
                      onClick={() => {
                        setError('');
                        setCustomModels((items) => items.filter((m) => m !== model));
                        onChange(models.filter((m) => m !== model));
                      }}
                    >
                      删除
                    </button>
                  )}
                  {onEditModel && selected && (
                    <button
                      className="text-button model-edit-action"
                      type="button"
                      disabled={disabled}
                      aria-label={`编辑 ${model} 能力`}
                      title="编辑模型能力"
                      onClick={() => onEditModel(model)}
                    >
                      编辑
                    </button>
                  )}
                  {!isDefault && selected && (
                    <button
                      className="text-button model-default-action"
                      type="button"
                      disabled={disabled}
                      aria-label={`将 ${model} 设为默认`}
                      onClick={() => onChange([model, ...models.filter((m) => m !== model)])}
                    >
                      设为默认
                    </button>
                  )}
                </span>
              </div>
            );
          })}
        </div>
      )}
      <div className="custom-model-entry">
        <input
          className="input"
          aria-label="添加自定义模型"
          placeholder="其他模型 ID"
          maxLength={200}
          value={draft}
          disabled={disabled}
          onChange={(e) => {
            setDraft(e.target.value);
            setError('');
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              addCustom();
            }
          }}
        />
        <button
          className="secondary"
          type="button"
          disabled={disabled || !draft.trim()}
          onClick={addCustom}
        >
          添加模型
        </button>
      </div>
      {error && (
        <span className="field-hint" role="alert">
          {error}
        </span>
      )}
      {draft.trim() && <span className="field-hint">点击「添加模型」或回车加入选择。</span>}
    </div>
  );
}
