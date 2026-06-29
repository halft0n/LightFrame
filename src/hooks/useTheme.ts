import { useEffect } from "react";
import { getSnapshot, setTheme, subscribe, type Theme } from "@/store/appStore";

const STORAGE_KEY = "lightframe-theme";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyThemeClass(theme: Theme) {
  const root = document.documentElement;
  const resolved = theme === "system" ? getSystemTheme() : theme;
  root.classList.remove("light", "dark");
  root.classList.add(resolved);
}

export function useTheme() {
  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY) as Theme | null;
    if (stored === "light" || stored === "dark" || stored === "system") {
      setTheme(stored);
    }

    const apply = () => applyThemeClass(getSnapshot().theme);
    apply();

    const unsubscribe = subscribe(apply);

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const onSystemChange = () => {
      if (getSnapshot().theme === "system") applyThemeClass("system");
    };
    mediaQuery.addEventListener("change", onSystemChange);

    return () => {
      unsubscribe();
      mediaQuery.removeEventListener("change", onSystemChange);
    };
  }, []);
}

export function changeTheme(theme: Theme) {
  localStorage.setItem(STORAGE_KEY, theme);
  setTheme(theme);
  applyThemeClass(theme);
}
