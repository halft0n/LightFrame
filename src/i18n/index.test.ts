import { describe, it, expect, beforeEach } from "vitest";
import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";
import { t, getLocale, setLocale, subscribe } from "./index";

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
});

describe("i18n", () => {
  it("defaults to zh-CN", () => {
    expect(getLocale()).toBe("zh-CN");
  });

  it("translates keys in zh-CN", () => {
    expect(t("app.title")).toBe("影迹");
    expect(t("sidebar.allPhotos")).toBe("所有照片");
  });

  it("switches to English", () => {
    setLocale("en");
    expect(getLocale()).toBe("en");
    expect(t("app.title")).toBe("LightFrame");
    expect(t("sidebar.allPhotos")).toBe("All Photos");
  });

  it("returns key for unknown translations", () => {
    expect(t("nonexistent.key")).toBe("nonexistent.key");
  });

  it("persists locale in localStorage", () => {
    setLocale("en");
    expect(localStorage.getItem("lightframe-locale")).toBe("en");
  });

  it("notifies subscribers on locale change", () => {
    let callCount = 0;
    const unsub = subscribe(() => callCount++);

    setLocale("en");
    expect(callCount).toBe(1);

    setLocale("zh-CN");
    expect(callCount).toBe(2);

    unsub();
    setLocale("en");
    expect(callCount).toBe(2);
  });

  it("all zh-CN keys exist in en", () => {
    setLocale("zh-CN");
    const zhKeys = [
      "app.title", "sidebar.library", "sidebar.allPhotos", "sidebar.timeline",
      "sidebar.locations", "sidebar.people", "sidebar.tools", "sidebar.duplicates",
      "sidebar.screenshots", "sidebar.settings", "sidebar.albums",
      "main.welcome", "main.addFolder",
    ];

    for (const key of zhKeys) {
      setLocale("zh-CN");
      const zh = t(key);
      setLocale("en");
      const enVal = t(key);
      expect(zh).not.toBe(key);
      expect(enVal).not.toBe(key);
    }
  });

  it("all locale keys match between zh-CN and en", () => {
    const zhKeys = Object.keys(zhCN).sort();
    const enKeys = Object.keys(en).sort();
    expect(zhKeys).toEqual(enKeys);

    for (const key of zhKeys) {
      setLocale("zh-CN");
      const zh = t(key);
      setLocale("en");
      const enVal = t(key);
      expect(zh).not.toBe(key);
      expect(enVal).not.toBe(key);
      expect(typeof zh).toBe("string");
      expect(typeof enVal).toBe("string");
    }
  });

  it("includes recently added translation keys", () => {
    const newKeys = [
      "deleted.confirmPermanent",
      "theme.subtitle",
      "gallery.loadError",
    ] as const;

    for (const key of newKeys) {
      setLocale("zh-CN");
      expect(t(key)).not.toBe(key);
      setLocale("en");
      expect(t(key)).not.toBe(key);
    }

    setLocale("zh-CN");
    expect(t("deleted.confirmPermanent")).toContain("永久删除");
    expect(t("theme.subtitle")).toContain("主题");
    expect(t("gallery.loadError")).toContain("加载");

    setLocale("en");
    expect(t("deleted.confirmPermanent")).toContain("Permanently delete");
    expect(t("theme.subtitle")).toContain("appearance");
    expect(t("gallery.loadError")).toContain("Failed");
  });
});
