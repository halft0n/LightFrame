import { describe, it, expect } from "vitest";
import { t, setLocale } from "@/i18n/index";
import { localizeError } from "./errors";

describe("localizeError", () => {
  it("maps known backend error patterns to i18n keys", () => {
    setLocale("zh-CN");
    expect(localizeError(new Error("media 1 not found"), t)).toBe("资源未找到");
    expect(localizeError("database locked", t)).toBe("数据库错误，请重试");
    expect(localizeError("permission denied", t)).toBe("没有访问权限");
    expect(localizeError("file too large", t)).toBe("文件过大");
    expect(localizeError("batch size exceeded", t)).toBe("批量操作数量超出限制");
    expect(localizeError("something else", t)).toBe("操作失败，请重试");
  });
});
