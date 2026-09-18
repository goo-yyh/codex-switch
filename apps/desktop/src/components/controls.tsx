import { useI18n } from '../i18n';
import * as Select from '@radix-ui/react-select';
import * as Dialog from '@radix-ui/react-dialog';
import { Check, ChevronDown, X } from 'lucide-react';
import type { ReactNode } from 'react';
export function Picker({
  value,
  onChange,
  options,
  label,
  id,
  disabled,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
  label: string;
  id?: string;
  disabled?: boolean;
  placeholder?: string;
}) {
  return (
    <Select.Root disabled={disabled} value={value} onValueChange={onChange}>
      <Select.Trigger id={id} className="input picker" aria-label={label}>
        <Select.Value placeholder={placeholder} />
        <Select.Icon className="picker-icon">
          <ChevronDown size={16} />
        </Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        <Select.Content className="select-menu" position="popper" sideOffset={5}>
          <Select.Viewport>
            {options.map((o) => (
              <Select.Item className="select-option" key={o.value} value={o.value}>
                <Select.ItemText>{o.label}</Select.ItemText>
                <Select.ItemIndicator>
                  <Check size={14} />
                </Select.ItemIndicator>
              </Select.Item>
            ))}
          </Select.Viewport>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
  );
}
export function Modal({
  open,
  onClose,
  title,
  description,
  error,
  busy = false,
  dismissible = true,
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description: string;
  error?: string;
  busy?: boolean;
  dismissible?: boolean;
  children: ReactNode;
}) {
  const { t, message } = useI18n();
  return (
    <Dialog.Root open={open} onOpenChange={(v) => !v && !busy && dismissible && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="overlay" />
        <Dialog.Content
          className="dialog"
          onEscapeKeyDown={(event) => {
            if (!dismissible) event.preventDefault();
          }}
          onInteractOutside={(event) => {
            if (!dismissible) event.preventDefault();
          }}
        >
          <div className="dialog-heading">
            <Dialog.Title>{title}</Dialog.Title>
            {dismissible && (
              <Dialog.Close className="icon-button" aria-label={t('关闭')} disabled={busy}>
                <X size={19} />
              </Dialog.Close>
            )}
          </div>
          <Dialog.Description className="muted">{description}</Dialog.Description>
          {error && (
            <div className="feedback error" role="alert">
              {message(error)}
            </div>
          )}
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
