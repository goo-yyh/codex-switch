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
    refresh().catch((e) => setError(String(e)));
    const onFocus = () => {
      if (!busyRef.current) refresh().catch((e) => setError(String(e)));
    };
    window.addEventListener('focus', onFocus);
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    subscribeToAppChanges(onFocus)
      .then((stop) => {
        if (disposed) stop();
        else {
          unsubscribe = stop;
          // Catch a startup registry update that finished before listeners were ready.
          onFocus();
        }
      })
      .catch((e) => setError(String(e)));
    return () => {
      disposed = true;
      unsubscribe?.();
      window.removeEventListener('focus', onFocus);
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
