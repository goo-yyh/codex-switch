import * as Switch from '@radix-ui/react-switch';
import { ArrowLeft, ShieldCheck } from 'lucide-react';
import { call, type Snapshot } from '../../api/bridge';
import type { AppController } from '../../hooks/useAppController';
import { CompactionPreferences } from './CompactionPreferences';
import { product } from '../../product';

export function SettingsPage({
  data,
  controller,
  onBack,
}: {
  data: Snapshot;
  controller: AppController;
  onBack: () => void;
}) {
  const { busy, run } = controller;
  return (
    <section className="settings-page">
      <button className="text-button back" disabled={busy} onClick={() => onBack()}>
        <ArrowLeft size={15} />
        返回
      </button>
      <h1>通用设置</h1>
      <h2 className="settings-section-title">应用设置</h2>
      <div className="settings-group">
        <CompactionPreferences
          settings={
            data.routingSettings ?? {
              remoteCompaction: false,
            }
          }
          disabled={busy || data.enabled}
          onChange={(settings) =>
            run(async () => {
              await call('save_routing_settings', { settings });
            })
          }
        />
        <div className="setting-row">
          <div>
            <h3>登录电脑时启动</h3>
            <p className="muted small">默认关闭，由你决定何时启动。</p>
          </div>
          <Switch.Root
            className="switch"
            checked={data.autostart}
            disabled={busy || data.enabled}
            onCheckedChange={(enabled) =>
              run(async () => {
                await call('set_autostart', { enabled });
              })
            }
            aria-label="登录电脑时启动"
          >
            <Switch.Thumb className="switch-thumb" />
          </Switch.Root>
        </div>
        <div className="setting-row">
          <div>
            <h3>Codex App</h3>
            <p className="muted small">
              {data.app.installed ? '已找到应用' : '未找到应用'} ·{' '}
              {data.app.running ? '正在运行' : '未运行'}
            </p>
          </div>
          <button className="text-button" disabled={busy} onClick={() => run(async () => {})}>
            重新检测
          </button>
        </div>
      </div>
      <h2 className="settings-section-title">配置与恢复</h2>
      <div className="settings-group">
        <div className="setting-row">
          <div>
            <h3>配置位置</h3>
            <p className="muted small path">{data.configPath}</p>
          </div>
        </div>
        <div className="setting-row">
          <div>
            <h3>恢复开启前的配置</h3>
            <p className="muted small">关闭开关即可恢复。你的登录和会话保持原样。</p>
          </div>
        </div>
      </div>
      <div className="settings-note">
        <ShieldCheck size={20} />
        <div>
          <h3>数据留在你的设备</h3>
          <p className="muted small">
            Key 存入系统凭据库。请求由本机发送到所选服务，我们不提供云端中转。
          </p>
        </div>
      </div>
      <button
        className="secondary full"
        disabled={busy}
        onClick={() =>
          run(async () => {
            await call('quit');
          })
        }
      >
        完全退出 Codex Switch
      </button>
      <p className="form-footnote">关闭窗口会保留后台连接。完全退出前请先正常退出 Codex。</p>
      <p className="about">Codex Switch {product.version} · 社区独立开源项目</p>
    </section>
  );
}
