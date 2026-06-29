import { describe, it, expect } from "vitest";
import { escapeHtml } from "./escapeHtml";

describe("escapeHtml", () => {
  it("escapes ampersand, angle brackets, and quotes", () => {
    expect(escapeHtml(`&<>"'`)).toBe("&amp;&lt;&gt;&quot;&#039;");
  });

  it("handles empty string", () => {
    expect(escapeHtml("")).toBe("");
  });

  it("handles string with no special characters", () => {
    expect(escapeHtml("hello world 123")).toBe("hello world 123");
  });

  it("handles string with all special characters", () => {
    expect(escapeHtml(`a&b<c>d"e'f`)).toBe("a&amp;b&lt;c&gt;d&quot;e&#039;f");
  });

  it("handles unicode characters unchanged", () => {
    expect(escapeHtml("你好 🌅 & <")).toBe("你好 🌅 &amp; &lt;");
  });
});
