import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";

export type Locale = "zh-CN" | "en";
export type TranslationKey = keyof typeof zhCN;

const translations: Record<Locale, Record<string, string>> = {
  "zh-CN": zhCN,
  en,
};

function detectLocale(): Locale {
  const stored = localStorage.getItem("catchlight-locale");
  if (stored && stored in translations) return stored as Locale;

  const browserLang = navigator.language;
  if (browserLang.startsWith("zh")) return "zh-CN";
  return "en";
}

let currentLocale: Locale = detectLocale();
const listeners = new Set<() => void>();

export function getLocale(): Locale {
  return currentLocale;
}

export function setLocale(locale: Locale) {
  currentLocale = locale;
  localStorage.setItem("catchlight-locale", locale);
  listeners.forEach((fn) => fn());
}

export function t(key: string): string {
  return translations[currentLocale][key] ?? key;
}

export function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
