import { useEffect, useState } from 'react';
import { Download } from 'lucide-react';
import { call, type AvailableUpdate } from '../../api/bridge';
import { useI18n } from '../../i18n';
import type { AppController } from '../../hooks/useAppController';

export function UpdateButton({ disabled, run }: { disabled: boolean; run: AppController['run'] }) {
  const { t } = useI18n();
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);
  useEffect(() => {
    let disposed = false;
    let checking = false;
    async function check() {
      if (checking) return;
      checking = true;
      try {
        const result = await call<AvailableUpdate | null>('check_update');
        if (!disposed) setUpdate(result ?? null);
      } catch {
        // A release check must not interrupt configuration work. Keep an existing
        // download hint if a later check fails; the native layer throttles retries.
      } finally {
        checking = false;
      }
    }
    void check();
    window.addEventListener('focus', check);
    // Native caching limits successful network checks to once every six hours.
    const timer = window.setInterval(check, 15 * 60 * 1000);
    return () => {
      disposed = true;
      window.removeEventListener('focus', check);
      window.clearInterval(timer);
    };
  }, []);

  if (!update) return null;
  return (
    <button
      className="header-link update-download"
      disabled={disabled}
      aria-label={t('下载新版本 {version}', { version: update.version })}
      title={t('发现新版本 {version}，点击下载安装包', { version: update.version })}
      onClick={() =>
        run(async () => {
          await call('open_link', { url: update.url });
        })
      }
    >
      <span className="update-download-icon" aria-hidden="true">
        <Download size={17} />
      </span>
      <span>v{update.version}</span>
    </button>
  );
}
