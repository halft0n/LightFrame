import { describe, it, expect, beforeEach } from "vitest";
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
    expect(t("app.title")).toBe("拾光");
    expect(t("sidebar.allPhotos")).toBe("所有照片");
  });

  it("switches to English", () => {
    setLocale("en");
    expect(getLocale()).toBe("en");
    expect(t("app.title")).toBe("CatchLight");
    expect(t("sidebar.allPhotos")).toBe("All Photos");
  });

  it("returns key for unknown translations", () => {
    expect(t("nonexistent.key")).toBe("nonexistent.key");
  });

  it("persists locale in localStorage", () => {
    setLocale("en");
    expect(localStorage.getItem("catchlight-locale")).toBe("en");
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
      const en = t(key);
      expect(zh).not.toBe(key);
      expect(en).not.toBe(key);
    }
  });
});
