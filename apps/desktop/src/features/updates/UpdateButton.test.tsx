import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { UpdateButton } from './UpdateButton';
import { call } from '../../api/bridge';
import { LocaleContext } from '../../i18n';
vi.mock('../../api/bridge', () => ({ call: vi.fn() }));
const update = {
  version: '0.2.0',
  url: 'https://github.com/goo-yyh/codex-switch/releases/download/v0.2.0/Codex_0.2.0_aarch64.dmg',
};
const run = vi.fn(async (action: () => Promise<void>) => {
  await action();
});
beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('release download hint', () => {
  it('shows only an available update and opens its installer URL without starting the service', async () => {
    vi.mocked(call).mockResolvedValue(update);
    render(<UpdateButton disabled={false} run={run} />);
    const button = await screen.findByRole('button', { name: '下载新版本 0.2.0' });
    expect(button).toHaveAttribute('title', '发现新版本 0.2.0，点击下载安装包');
    fireEvent.click(button);
    await waitFor(() => expect(call).toHaveBeenCalledWith('open_link', { url: update.url }));
    expect(vi.mocked(call).mock.calls.map(([command]) => command)).toEqual([
      'check_update',
      'open_link',
    ]);
  });
  it.each([null, new Error('offline')])(
    'stays quiet without an available result: %s',
    async (result) => {
      vi.mocked(call).mockImplementation(async () => {
        if (result instanceof Error) throw result;
        return result as never;
      });
      await act(async () => {
        render(<UpdateButton disabled={false} run={run} />);
      });
      expect(call).toHaveBeenCalledWith('check_update');
      expect(screen.queryByRole('button')).not.toBeInTheDocument();
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    },
  );
  it('follows language changes and stays disabled while the service is locked', async () => {
    vi.mocked(call).mockResolvedValue(update);
    const view = render(<UpdateButton disabled={false} run={run} />);
    await screen.findByRole('button', { name: '下载新版本 0.2.0' });
    view.rerender(
      <LocaleContext.Provider value="en">
        <UpdateButton disabled run={run} />
      </LocaleContext.Provider>,
    );
    const button = await screen.findByRole('button', { name: 'Download version 0.2.0' });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(call).not.toHaveBeenCalledWith('open_link', expect.anything());
  });
  it('rechecks on focus and on a timer, preserves a hint on failure, and cleans up', async () => {
    vi.useFakeTimers();
    vi.mocked(call).mockResolvedValue(null);
    const view = render(<UpdateButton disabled={false} run={run} />);
    await act(async () => {});
    vi.mocked(call).mockResolvedValue(update);
    await act(async () => {
      fireEvent.focus(window);
    });
    expect(screen.getByRole('button', { name: '下载新版本 0.2.0' })).toBeInTheDocument();
    vi.mocked(call).mockRejectedValue(new Error('offline'));
    await act(async () => {
      vi.advanceTimersByTime(15 * 60 * 1000);
    });
    expect(screen.getByRole('button', { name: '下载新版本 0.2.0' })).toBeInTheDocument();
    expect(call).toHaveBeenCalledTimes(3);
    view.unmount();
    fireEvent.focus(window);
    vi.advanceTimersByTime(15 * 60 * 1000);
    expect(call).toHaveBeenCalledTimes(3);
  });
});
