import { describe, it, expect } from "vitest";
import { getThumbnailUrl } from "@/lib/tauri";

describe("getThumbnailUrl", () => {
  it("generates valid URLs for different sizes", () => {
    const small = getThumbnailUrl(42, "small");
    const large = getThumbnailUrl(42, "large");
    const micro = getThumbnailUrl(42, "micro");

    expect(small).toBeTruthy();
    expect(large).toBeTruthy();
    expect(micro).toBeTruthy();
    expect(small).not.toBe(large);
    expect(small).not.toBe(micro);
  });

  it("defaults to small size", () => {
    const url = getThumbnailUrl(7);
    expect(url).toBeTruthy();
    expect(url).toBe(getThumbnailUrl(7, "small"));
  });
});
