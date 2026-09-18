import { createContext, useContext } from 'react';
import { localizeMessage } from './messages';
import { en } from './en';

export type Locale = 'zh-CN' | 'en';
export type Message = keyof typeof en;
export const LocaleContext = createContext<Locale>('zh-CN');
type Values = Record<string, string | number>;

export function translate(locale: Locale, key: Message, values: Values = {}) {
  const message = locale === 'en' ? en[key] : key;
  return message.replace(/\{(\w+)\}/g, (placeholder, name) => String(values[name] ?? placeholder));
}

// Only catalogued display metadata is translated. User-authored names, model IDs,
// URLs and values are never sent through substring replacements.
export function displayText(locale: Locale, value: string) {
  return locale === 'en' && Object.hasOwn(en, value) ? en[value as Message] : value;
}

export function useI18n() {
  const locale = useContext(LocaleContext);
  return {
    locale,
    message: (value: string) => localizeMessage(locale, value),
    t: (key: Message, values?: Values) => translate(locale, key, values),
    text: (value: string) => displayText(locale, value),
  };
}
