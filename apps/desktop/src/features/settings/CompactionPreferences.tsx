import * as Switch from '@radix-ui/react-switch';
import type { RoutingSettings } from '../../api/bridge';
export function CompactionPreferences({
  settings,
  disabled,
  onChange,
}: {
  settings: RoutingSettings;
  disabled: boolean;
  onChange: (s: RoutingSettings) => void;
}) {
  return (
    <div className="setting-row" title={disabled ? '请先关闭 Codex Switch 后再修改' : undefined}>
      <div>
        <h3>远程上下文压缩</h3>
        <p id="compaction-description" className="muted small">
          由服务端压缩上下文，需服务支持，建议不开启。
        </p>
      </div>
      <Switch.Root
        className="switch"
        aria-label="远程上下文压缩"
        aria-describedby="compaction-description"
        checked={settings.remoteCompaction}
        disabled={disabled}
        onCheckedChange={(remoteCompaction) => onChange({ remoteCompaction })}
      >
        <Switch.Thumb className="switch-thumb" />
      </Switch.Root>
    </div>
  );
}
