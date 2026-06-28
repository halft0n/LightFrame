import { describe, it, expect } from "vitest";
import { getOriginalUrl, getThumbnailUrl } from "@/lib/tauri";

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

describe("getOriginalUrl", () => {
  it("generates original protocol URLs with encoded path", () => {
    expect(getOriginalUrl("/home/user/photo.jpg")).toBe(
      "original://localhost/%2Fhome%2Fuser%2Fphoto.jpg",
    );
  });

  it("encodes spaces and special characters", () => {
    expect(getOriginalUrl("/photos/my photo (1).jpg")).toBe(
      "original://localhost/%2Fphotos%2Fmy%20photo%20(1).jpg",
    );
  });
});
