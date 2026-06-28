import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import {
  getOriginalUrl,
  getThumbnailUrl,
  getFavoriteState,
  getMediaList,
  getMediaCount,
  getMediaById,
  toggleFavorite,
  saveEdit,
  getEdit,
  addWatchedFolder,
  listWatchedFolders,
} from "@/lib/tauri";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("getThumbnailUrl", () => {
  it("generates thumb protocol URLs for all sizes", () => {
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
    expect(getOriginalUrl("/path/with#hash&query=1")).toBe(
      "original://localhost/%2Fpath%2Fwith%23hash%26query%3D1",
    );
  });
});

describe("getFavoriteState", () => {
  it("returns true when invoke succeeds with true", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(getFavoriteState(1)).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("is_favorite", { mediaId: 1 });
  });

  it("returns false when invoke succeeds with false", async () => {
    mockInvoke.mockResolvedValue(false);
    await expect(getFavoriteState(2)).resolves.toBe(false);
  });

  it("returns false when invoke throws synchronously", async () => {
    mockInvoke.mockImplementation(() => {
      throw new Error("backend unavailable");
    });
    await expect(getFavoriteState(3)).resolves.toBe(false);
  });

  it("returns false when invoke rejects", async () => {
    mockInvoke.mockRejectedValue(new Error("backend unavailable"));
    await expect(getFavoriteState(4)).rejects.toThrow("backend unavailable");
  });
});

describe("async tauri functions", () => {
  it("getMediaList invokes with offset and limit", async () => {
    const items = [{ id: 1, filename: "a.jpg" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaList(10, 60)).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_list", { offset: 10, limit: 60 });
  });

  it("getMediaCount invokes get_media_count", async () => {
    mockInvoke.mockResolvedValue(100);
    await expect(getMediaCount()).resolves.toBe(100);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_count");
  });

  it("getMediaById invokes with id", async () => {
    const item = { id: 5, filename: "test.jpg" };
    mockInvoke.mockResolvedValue(item);
    await expect(getMediaById(5)).resolves.toEqual(item);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_by_id", { id: 5 });
  });

  it("toggleFavorite invokes and returns boolean", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(toggleFavorite(7)).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("toggle_favorite", { mediaId: 7 });
  });

  it("saveEdit invokes with mediaId and params", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await saveEdit(1, '{"brightness":10}');
    expect(mockInvoke).toHaveBeenCalledWith("save_edit", {
      mediaId: 1,
      params: '{"brightness":10}',
    });
  });

  it("getEdit invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue('{"brightness":5}');
    await expect(getEdit(3)).resolves.toBe('{"brightness":5}');
    expect(mockInvoke).toHaveBeenCalledWith("get_edit", { mediaId: 3 });
  });

  it("addWatchedFolder invokes with path", async () => {
    const folder = { id: 1, path: "/photos", media_count: 0, scan_status: "idle" };
    mockInvoke.mockResolvedValue(folder);
    await expect(addWatchedFolder("/photos")).resolves.toEqual(folder);
    expect(mockInvoke).toHaveBeenCalledWith("add_watched_folder", { path: "/photos" });
  });

  it("listWatchedFolders invokes list_watched_folders", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listWatchedFolders()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_watched_folders");
  });
});
