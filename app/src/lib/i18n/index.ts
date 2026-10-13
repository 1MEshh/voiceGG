import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import en from './en.json';
import ar from './ar.json';

export type Locale = 'en' | 'ar';

export const locale = writable<Locale>('en');

const dictionaries: Record<Locale, Record<string, any>> = {
  en,
  ar,
};

function getNestedValue(obj: any, path: string): string | undefined {
  const parts = path.split('.');
  let current: any = obj;
  for (const part of parts) {
    if (current && typeof current === 'object' && part in current) {
      current = current[part];
    } else {
      return undefined;
    }
  }
  return typeof current === 'string' ? current : undefined;
}

export function translate(currentLocale: Locale, key: string, params?: Record<string, string | number>): string {
  let text = getNestedValue(dictionaries[currentLocale], key);
  if (text === undefined) {
    text = getNestedValue(dictionaries.en, key);
  }
  if (text === undefined) {
    return key;
  }
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
    }
  }
  return text;
}

export const t = derived(locale, ($locale) => {
  return (key: string, params?: Record<string, string | number>): string => {
    return translate($locale, key, params);
  };
});

export async function setLanguage(lang: 'en' | 'ar') {
  locale.set(lang);
  if (typeof document !== 'undefined') {
    document.documentElement.lang = lang;
    document.documentElement.dir = lang === 'ar' ? 'rtl' : 'ltr';
  }
  try {
    await invoke('set_language', { language: lang });
  } catch (e) {
    console.warn('Could not persist language via IPC:', e);
  }
}
