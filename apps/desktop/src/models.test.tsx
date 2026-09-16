import { useState } from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ModelSelection } from './ModelSelection';
import registry from '../../../packages/provider-registry/providers.json';
import models from '../../../packages/provider-registry/models.json';
function EditableModels({
  initial = [],
  disabled = false,
}: {
  initial?: string[];
  disabled?: boolean;
}) {
  const [selected, setSelected] = useState(initial);
  return (
    <ModelSelection
      provider="kimi"
      models={selected}
      onChange={setSelected}
      multiple
      disabled={disabled}
    />
  );
}
describe('model choices inside a profile', () => {
  it.each(registry)('$name has matching model choices', (p) => {
    render(
      <ModelSelection
        provider={p.id}
        models={[p.model]}
        onChange={() => {}}
        multiple
        disabled={false}
      />,
    );
    for (const model of models[p.id as keyof typeof models])
      expect(screen.getByRole('checkbox', { name: model })).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: p.model })).toBeChecked();
  });
  it('uses selection order for the default and lets another selected model become default', () => {
    render(<EditableModels />);
    const first = screen.getByRole('checkbox', { name: 'kimi-k3' });
    const second = screen.getByRole('checkbox', { name: 'kimi-k2.6' });
    fireEvent.click(first);
    fireEvent.click(second);
    expect(first).toHaveAccessibleDescription('默认');
    expect(screen.queryByText('已配置')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '将 kimi-k2.6 设为默认' }));
    expect(second).toHaveAccessibleDescription('默认');
    expect(first).toBeChecked();
    expect(second).toBeChecked();
    fireEvent.click(second);
    expect(first).toHaveAccessibleDescription('默认');
  });
  it('retains unchecked custom choices and removes custom defaults with a fallback', () => {
    render(<EditableModels initial={['private-model', 'kimi-k3']} />);
    expect(screen.queryByRole('button', { name: '删除模型 kimi-k3' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('checkbox', { name: 'private-model' }));
    expect(screen.getByRole('checkbox', { name: 'private-model' })).not.toBeChecked();
    const input = screen.getByLabelText('添加自定义模型');
    fireEvent.change(input, { target: { value: ' private-model ' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(screen.getByRole('alert')).toHaveTextContent('ID 已存在');
    fireEvent.click(screen.getByRole('checkbox', { name: 'private-model' }));
    fireEvent.click(screen.getByRole('button', { name: '将 private-model 设为默认' }));
    fireEvent.click(screen.getByRole('button', { name: '删除模型 private-model' }));
    expect(screen.queryByRole('checkbox', { name: 'private-model' })).not.toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toHaveAccessibleDescription('默认');
    fireEvent.change(input, { target: { value: ' kimi-k2.6 ' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(screen.getByRole('alert')).toHaveTextContent('ID 已存在');
    expect(screen.getByRole('checkbox', { name: 'kimi-k2.6' })).not.toBeChecked();
  });
  it('adds a unique custom model as the first default and can delete the last selection', () => {
    render(<EditableModels />);
    fireEvent.change(screen.getByLabelText('添加自定义模型'), { target: { value: ' custom-id ' } });
    fireEvent.click(screen.getByRole('button', { name: '添加模型' }));
    expect(screen.getByRole('checkbox', { name: 'custom-id' })).toHaveAccessibleDescription('默认');
    fireEvent.click(screen.getByRole('button', { name: '删除模型 custom-id' }));
    expect(screen.queryByRole('checkbox', { name: 'custom-id' })).not.toBeInTheDocument();
    expect(screen.queryByText('默认')).not.toBeInTheDocument();
  });
  it('pins new custom and selected models without changing the default', () => {
    render(<EditableModels initial={['kimi-k3']} />);
    const displayed = () =>
      screen.getAllByRole('checkbox').map((item) => item.getAttribute('aria-label'));
    for (const model of ['custom-one', 'custom-two']) {
      fireEvent.change(screen.getByLabelText('添加自定义模型'), { target: { value: model } });
      fireEvent.click(screen.getByRole('button', { name: '添加模型' }));
      expect(screen.getAllByRole('checkbox')[0]).toHaveAccessibleName(model);
      expect(screen.getAllByRole('checkbox')[0]).toBeChecked();
    }
    fireEvent.click(screen.getByRole('checkbox', { name: 'kimi-k2.6' }));
    expect(displayed().slice(0, 4)).toEqual(['custom-two', 'custom-one', 'kimi-k3', 'kimi-k2.6']);
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toHaveAccessibleDescription('默认');
    fireEvent.click(screen.getByRole('checkbox', { name: 'custom-two' }));
    expect(displayed().slice(0, 4)).toEqual(['custom-one', 'kimi-k3', 'kimi-k2.6', 'custom-two']);
    expect(screen.getByRole('checkbox', { name: 'kimi-k3' })).toHaveAccessibleDescription('默认');
  });
  it('locks default and delete actions during validation', () => {
    render(<EditableModels initial={['kimi-k3', 'custom-id']} disabled />);
    expect(screen.getByRole('button', { name: '将 custom-id 设为默认' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '删除模型 custom-id' })).toBeDisabled();
  });
  it('trims custom models and rejects duplicate input', () => {
    const change = vi.fn();
    render(
      <ModelSelection
        provider="custom"
        models={['same']}
        onChange={change}
        multiple
        disabled={false}
      />,
    );
    const input = screen.getByLabelText('添加自定义模型');
    fireEvent.change(input, { target: { value: ' same ' } });
    fireEvent.click(screen.getByRole('button', { name: '添加模型' }));
    expect(screen.getByRole('alert')).toHaveTextContent('ID 已存在');
    expect(change).not.toHaveBeenCalled();
    fireEvent.change(input, { target: { value: ' new ' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(change).toHaveBeenCalledWith(['same', 'new']);
  });
  it('preserves unknown saved models on edit', () => {
    render(
      <ModelSelection
        provider="kimi"
        models={['private-model']}
        onChange={() => {}}
        multiple
        disabled={false}
      />,
    );
    expect(screen.getByRole('checkbox', { name: 'private-model' })).toBeChecked();
  });
});

it('uses the selected package model list without mixing the public API list', () => {
  render(
    <ModelSelection
      provider="kimi"
      availableModels={['kimi-for-coding', 'k3']}
      models={['kimi-for-coding']}
      onChange={() => {}}
      multiple
      disabled={false}
    />,
  );
  expect(screen.getByRole('checkbox', { name: 'kimi-for-coding' })).toBeChecked();
  expect(screen.getByRole('checkbox', { name: 'k3' })).toBeInTheDocument();
  expect(screen.queryByRole('checkbox', { name: 'kimi-k3' })).not.toBeInTheDocument();
});
