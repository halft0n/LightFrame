import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";

export type Locale = "zh-CN" | "en";
export type TranslationKey = keyof typeof zhCN;

const translations: Record<Locale, Record<string, string>> = {
  "zh-CN": zhCN,
  en,
};

function detectLocale(): Locale {
  const stored = localStorage.getItem("lightframe-locale");
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
  localStorage.setItem("lightframe-locale", locale);
  listeners.forEach((fn) => fn());
}

export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  let text = translations[currentLocale][key] ?? key;
  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.replace(`{${name}}`, String(value));
    }
  }
  return text;
}

export function subscribe(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
