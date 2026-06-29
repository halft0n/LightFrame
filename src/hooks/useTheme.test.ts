import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { changeTheme } from "./useTheme";
import { getSnapshot, setTheme } from "@/store/appStore";

describe("changeTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("light", "dark");
    setTheme("system");
  });

  afterEach(() => {
    document.documentElement.classList.remove("light", "dark");
  });

  it("applies light theme class to document root", () => {
    changeTheme("light");
    expect(getSnapshot().theme).toBe("light");
    expect(document.documentElement.classList.contains("light")).toBe(true);
    expect(localStorage.getItem("lightframe-theme")).toBe("light");
  });

  it("applies dark theme class to document root", () => {
    changeTheme("dark");
    expect(getSnapshot().theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("persists system theme preference", () => {
    changeTheme("system");
    expect(getSnapshot().theme).toBe("system");
    expect(localStorage.getItem("lightframe-theme")).toBe("system");
  });
});
