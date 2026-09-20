import { useEffect, useRef, useState } from 'react';
import { call, subscribeToAppChanges, type Snapshot } from '../api/bridge';

// Serialize user actions across pages; refresh again after failures because native
// recovery may have restored files before returning an error.
export function useAppController() {
  const [data, setData] = useState<Snapshot>();
  const [phase, setPhase] = useState<'idle' | 'working' | 'testing'>('idle');
  const busy = phase !== 'idle';
  const busyRef = useRef(false);
  const [notice, setNotice] = useState('');
  const [error, setError] = useState('');
  async function refresh() {
    const next = await call<Snapshot>('snapshot');
    setData(next);
    if (next.trayFeedback) {
      const { message, isError } = next.trayFeedback;
      setError(isError ? message : '');
    }
  }
  useEffect(() => {
    let disposed = false;
    const onChange = () => {
      if (!disposed && !busyRef.current) refresh().catch((e) => setError(String(e)));
    };
    let unsubscribe: (() => void) | undefined;
    subscribeToAppChanges(onChange)
      .then((stop) => {
        if (disposed) stop();
        else {
          unsubscribe = stop;
        }
      })
      .catch((e) => {
        if (!disposed) setError(String(e));
      })
      // Subscribe first, then take one initial snapshot so startup updates aren't missed.
      // Window focus is not an application state change and must not spawn process checks.
      .finally(onChange);
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);
  async function run(action: () => Promise<void>) {
    if (busyRef.current) return;
    busyRef.current = true;
    setPhase('working');
    setError('');
    setNotice('');
    try {
      await action();
      await refresh();
    } catch (e) {
      const text = String(e);
      try {
        await refresh();
      } catch {
        // Snapshot recovery is best-effort; preserve the original action failure.
      }
      setError(text);
    } finally {
      busyRef.current = false;
      setPhase('idle');
    }
  }
  return { data, busy, busyRef, phase, setPhase, error, setError, notice, setNotice, refresh, run };
}
export type AppController = ReturnType<typeof useAppController>;
