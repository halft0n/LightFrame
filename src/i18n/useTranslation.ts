import { useSyncExternalStore, useCallback } from "react";
import { t as translate, subscribe, getLocale, setLocale, type Locale } from "./index";

export function useTranslation() {
  useSyncExternalStore(subscribe, getLocale);

  const t = useCallback((key: string) => translate(key), []);

  return { t, locale: getLocale(), setLocale } as const;
}

export type { Locale };
