import { describe, it, expect } from "vitest";
import { getThumbnailUrl } from "@/lib/tauri";

describe("getThumbnailUrl", () => {
  it("generates thumb protocol URLs for different sizes", () => {
    expect(getThumbnailUrl(42, "small")).toBe("thumb://localhost/42/small");
    expect(getThumbnailUrl(42, "large")).toBe("thumb://localhost/42/large");
    expect(getThumbnailUrl(42, "micro")).toBe("thumb://localhost/42/micro");
  });

  it("defaults to small size", () => {
    expect(getThumbnailUrl(7)).toBe("thumb://localhost/7/small");
  });
});
