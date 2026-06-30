import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn(
    (filePath: string, protocol: string = "asset") =>
      `${protocol}://localhost/${filePath}`,
  ),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { getOriginalUrl, getThumbnailUrl } from "@/lib/tauri";

describe("protocol URL generation", () => {
  describe("thumb:// URLs", () => {
    it("constructs thumb protocol URLs for all sizes", () => {
      expect(getThumbnailUrl(1, "micro")).toBe("thumb://localhost/1/micro");
      expect(getThumbnailUrl(99, "small")).toBe("thumb://localhost/99/small");
      expect(getThumbnailUrl(100, "large")).toBe("thumb://localhost/100/large");
    });

    it("defaults to small size", () => {
      expect(getThumbnailUrl(7)).toBe("thumb://localhost/7/small");
    });
  });

  describe("original:// URLs", () => {
    it("constructs original protocol URLs", () => {
      expect(getOriginalUrl("/tmp/photo.jpg")).toBe(
        "original://localhost/%2Ftmp%2Fphoto.jpg",
      );
    });

    it("encodes special characters", () => {
      expect(getOriginalUrl("/path/with#hash&query=1")).toBe(
        "original://localhost/%2Fpath%2Fwith%23hash%26query%3D1",
      );
      expect(getOriginalUrl("/photos/my photo (1).jpg")).toBe(
        "original://localhost/%2Fphotos%2Fmy%20photo%20(1).jpg",
      );
    });

    it("encodes Chinese characters in paths", () => {
      expect(getOriginalUrl("/photos/日落海滩/照片.jpg")).toBe(
        "original://localhost/%2Fphotos%2F%E6%97%A5%E8%90%BD%E6%B5%B7%E6%BB%A9%2F%E7%85%A7%E7%89%87.jpg",
      );
    });
  });
});
