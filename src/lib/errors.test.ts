import { describe, it, expect, beforeEach } from "vitest";
import { t, setLocale } from "@/i18n/index";
import { localizeError } from "./errors";

describe("localizeError", () => {
  beforeEach(() => {
    setLocale("zh-CN");
  });

  it("maps not found errors", () => {
    expect(localizeError(new Error("media 1 not found"), t)).toBe("资源未找到");
    expect(localizeError("folder not found", t)).toBe("资源未找到");
  });

  it("maps database errors", () => {
    expect(localizeError("database locked", t)).toBe("数据库错误，请重试");
    expect(localizeError("SQLite database error", t)).toBe("数据库错误，请重试");
  });

  it("maps permission and forbidden errors", () => {
    expect(localizeError("permission denied", t)).toBe("没有访问权限");
    expect(localizeError("forbidden path", t)).toBe("没有访问权限");
  });

  it("maps file too large errors", () => {
    expect(localizeError("file too large", t)).toBe("文件过大");
    expect(localizeError("payload too large", t)).toBe("文件过大");
  });

  it("maps batch size errors", () => {
    expect(localizeError("batch size 1001 exceeds maximum 1000", t)).toBe(
      "批量操作数量超出限制",
    );
  });

  it("falls back to generic error", () => {
    expect(localizeError("something else", t)).toBe("操作失败，请重试");
    expect(localizeError(42, t)).toBe("操作失败，请重试");
  });

  it("maps errors in English locale", () => {
    setLocale("en");
    expect(localizeError("media not found", t)).toBe("Resource not found");
    expect(localizeError("database locked", t)).toBe("Database error, please try again");
  });
});
