import { useEffect, useState } from 'react';
import { Button, DropdownMenu, Theme } from '@radix-ui/themes';
import { ArrowUpRight, Download } from 'lucide-react';
import '@radix-ui/themes/styles.css';
import './download-menu.css';
import release from '../../../../../packages/product-info/downloads.json';

interface Props {
  product: 'switch' | 'codex';
  lang: 'zh' | 'en';
}

export default function DownloadDropdown({ product, lang }: Props) {
  const [appearance, setAppearance] = useState<'light' | 'dark'>('light');
  useEffect(() => {
    const root = document.documentElement;
    const sync = () => setAppearance(root.dataset.theme === 'dark' ? 'dark' : 'light');
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(root, { attributes: true, attributeFilter: ['data-theme'] });
    return () => observer.disconnect();
  }, []);
  const en = lang === 'en';
  const isSwitch = product === 'switch';
  const label = `${en ? 'Download' : '下载'} ${isSwitch ? 'Codex Switch' : 'Codex App'}`;
  const base = `/downloads/v${release.version}`;
  const items = isSwitch
    ? [
        ['macOS · Apple Silicon', `${base}/Codex-Switch_aarch64.dmg`],
        ['macOS · Intel', `${base}/Codex-Switch_x64.dmg`],
        ['Windows · x64', `${base}/Codex-Switch_x64-setup.exe`],
      ]
    : [
        ['macOS · Apple Silicon', 'https://persistent.oaistatic.com/codex-app-prod/Codex.dmg'],
        ['macOS · Intel', 'https://persistent.oaistatic.com/codex-app-prod/Codex-latest-x64.dmg'],
        [
          'Windows',
          'https://get.microsoft.com/installer/download/9PLM9XGG6VKS?cid=website_cta_psi',
        ],
      ];
  return (
    <Theme
      appearance={appearance}
      accentColor="gray"
      radius="medium"
      className="download-theme"
      hasBackground={false}
    >
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <Button size="3" variant={isSwitch ? 'solid' : 'outline'} highContrast>
            {/* Codex icon: official desktop app resource icon-codex-light.png. */}
            <img
              src={isSwitch ? '/mark.png' : '/codex-app.png'}
              alt=""
              width={24}
              height={24}
              className="download-product-logo"
            />
            {label}
            <DropdownMenu.TriggerIcon />
          </Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content
          size="2"
          variant="soft"
          align="start"
          sideOffset={8}
          collisionPadding={16}
        >
          {items.map(([name, href]) => (
            <DropdownMenu.Item key={href} asChild>
              <a href={href} download={isSwitch || undefined}>
                <Download size={16} aria-hidden="true" />
                {name}
              </a>
            </DropdownMenu.Item>
          ))}
          <DropdownMenu.Separator />
          <DropdownMenu.Item asChild>
            <a href={isSwitch ? `${base}/SHA256SUMS.txt` : 'https://learn.chatgpt.com/docs/app'}>
              <ArrowUpRight size={16} aria-hidden="true" />
              {isSwitch
                ? en
                  ? 'SHA-256 checksums'
                  : 'SHA-256 校验文件'
                : en
                  ? 'Official installation guide'
                  : '官方安装指南'}
            </a>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </Theme>
  );
}
