import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import {
  protocolUrl,
  getOriginalUrl,
  getThumbnailUrl,
  getFaceThumbnailUrl,
  getFavoriteState,
  getMediaList,
  getMediaPage,
  getMediaCount,
  getMediaById,
  toggleFavorite,
  saveEdit,
  getEdit,
  addWatchedFolder,
  removeWatchedFolder,
  listWatchedFolders,
  getMediaByFolder,
  getMediaCountByFolder,
  batchExport,
  getMediaByType,
  getMediaCountByType,
  getTimelineGroups,
  getMediaNeighbors,
  getMediaWindow,
  scanFolder,
  onScanProgress,
  onFolderChanged,
  runDedupScan,
  getDuplicateGroups,
  getDuplicateCount,
  resolveDuplicate,
  dismissDuplicateGroup,
  getLocationGroups,
  getMediaByLocation,
  getLocationStats,
  createAlbum,
  deleteAlbum,
  updateAlbum,
  setAlbumCover,
  listAlbums,
  addToAlbum,
  removeFromAlbum,
  getAlbumMedia,
  getFavorites,
  getFavoritesCount,
  deleteMedia,
  getDeletedMedia,
  restoreMedia,
  permanentlyDelete,
  batchDeleteMedia,
  batchAddToAlbum,
  batchToggleFavorite,
  batchRestoreMedia,
  batchPermanentDelete,
  searchMedia,
  searchMediaCount,
  semanticSearch,
  createSmartAlbum,
  listSmartAlbums,
  deleteSmartAlbum,
  getSmartAlbumMedia,
  generateMemories,
  getOnThisDay,
  listMemories,
  getMemoryMedia,
  getAiStatus,
  getModelStatus,
  downloadModel,
  openModelsDir,
  getScreenshots,
  getScreenshotCount,
  computeClipEmbedding,
  computeClipEmbeddingsBatch,
  findSimilarPhotos,
  detectFaces,
  detectFacesBatch,
  onFaceDetectionProgress,
  getFaces,
  getPersonFaces,
  listPersons,
  getPersonMedia,
  renamePerson,
  clusterFaces,
  mergePersons,
  splitFaceFromPerson,
  revertEdit,
  hasEdits,
  exportEdited,
  regenerateThumbnails,
  regenerateThumbnailSingle,
  onThumbnailRegenProgress,
  getMediaWithGeo,
  getGeoClusters,
} from "@/lib/tauri";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;
const mockListen = listen as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  mockListen.mockResolvedValue(() => {});
});

describe("protocolUrl", () => {
  it("uses scheme://localhost/ format on non-Windows", () => {
    expect(protocolUrl("thumb", "42/small")).toBe("thumb://localhost/42/small");
  });

  it("preserves path separators without encoding", () => {
    expect(protocolUrl("original", "C%3A%2FUsers%2Fphoto.jpg")).toBe(
      "original://localhost/C%3A%2FUsers%2Fphoto.jpg",
    );
    expect(protocolUrl("face", "7")).toBe("face://localhost/7");
  });
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

  it("normalizes Windows backslashes to forward slashes", () => {
    expect(getOriginalUrl("C:\\Users\\photo.jpg")).toBe(
      "original://localhost/C%3A%2FUsers%2Fphoto.jpg",
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
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    mockInvoke.mockRejectedValue(new Error("backend unavailable"));
    await expect(getFavoriteState(4)).resolves.toBe(false);
    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });
});

describe("async tauri functions", () => {
  it("getMediaList invokes with offset and limit", async () => {
    const items = [{ id: 1, filename: "a.jpg" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaList(10, 60)).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_list", {
      offset: 10,
      limit: 60,
    });
  });

  it("getMediaPage invokes with limit and cursor", async () => {
    const items = [{ id: 1, filename: "a.jpg", created_at: "2024-01-01" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaPage(60, ["2024-01-01", 1])).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_page", {
      limit: 60,
      cursor: ["2024-01-01", 1],
    });
  });

  it("getMediaPage invokes with null cursor when omitted", async () => {
    mockInvoke.mockResolvedValue([]);
    await getMediaPage(60);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_page", {
      limit: 60,
      cursor: null,
    });
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
    const folder = {
      id: 1,
      path: "/photos",
      media_count: 0,
      scan_status: "idle",
    };
    mockInvoke.mockResolvedValue(folder);
    await expect(addWatchedFolder("/photos")).resolves.toEqual(folder);
    expect(mockInvoke).toHaveBeenCalledWith("add_watched_folder", {
      path: "/photos",
    });
  });

  it("listWatchedFolders invokes list_watched_folders", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listWatchedFolders()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_watched_folders");
  });
});

describe("getFaceThumbnailUrl", () => {
  it("generates face protocol URLs", () => {
    expect(getFaceThumbnailUrl(42)).toBe("face://localhost/42");
  });
});

describe("removeWatchedFolder", () => {
  it("invokes remove_watched_folder with id", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await removeWatchedFolder(3);
    expect(mockInvoke).toHaveBeenCalledWith("remove_watched_folder", { id: 3 });
  });
});

describe("media by folder", () => {
  it("getMediaByFolder invokes with folderId, offset, and limit", async () => {
    const items = [{ id: 1, filename: "a.jpg" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaByFolder(2, 10, 50)).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_by_folder", {
      folderId: 2,
      offset: 10,
      limit: 50,
    });
  });

  it("getMediaCountByFolder invokes with folderId", async () => {
    mockInvoke.mockResolvedValue(25);
    await expect(getMediaCountByFolder(2)).resolves.toBe(25);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_count_by_folder", {
      folderId: 2,
    });
  });
});

describe("batchExport", () => {
  it("invokes batch_export with mediaIds and outputDir", async () => {
    mockInvoke.mockResolvedValue(3);
    await expect(batchExport([1, 2, 3], "/output")).resolves.toBe(3);
    expect(mockInvoke).toHaveBeenCalledWith("batch_export", {
      mediaIds: [1, 2, 3],
      outputDir: "/output",
    });
  });
});

describe("media by type", () => {
  it("getMediaByType invokes with mediaType, offset, and limit", async () => {
    const items = [{ id: 1, filename: "v.mp4" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaByType("Video", 0, 60)).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_by_type", {
      mediaType: "Video",
      offset: 0,
      limit: 60,
    });
  });

  it("getMediaCountByType invokes with mediaType", async () => {
    mockInvoke.mockResolvedValue(10);
    await expect(getMediaCountByType("Photo")).resolves.toBe(10);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_count_by_type", {
      mediaType: "Photo",
    });
  });
});

describe("getTimelineGroups", () => {
  it("invokes with default limit and null cursor", async () => {
    mockInvoke.mockResolvedValue([]);
    await getTimelineGroups();
    expect(mockInvoke).toHaveBeenCalledWith("get_timeline_groups", {
      limit: 200,
      cursorCreatedAt: null,
      cursorId: null,
    });
  });

  it("invokes with cursor when provided", async () => {
    mockInvoke.mockResolvedValue([]);
    await getTimelineGroups(100, { createdAt: "2024-01-01", id: 5 });
    expect(mockInvoke).toHaveBeenCalledWith("get_timeline_groups", {
      limit: 100,
      cursorCreatedAt: "2024-01-01",
      cursorId: 5,
    });
  });
});

describe("media navigation", () => {
  it("getMediaNeighbors invokes with id", async () => {
    const neighbors = { prev_id: 4, next_id: 6 };
    mockInvoke.mockResolvedValue(neighbors);
    await expect(getMediaNeighbors(5)).resolves.toEqual(neighbors);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_neighbors", { id: 5 });
  });

  it("getMediaWindow invokes with mediaId and radius", async () => {
    const items = [{ id: 5, filename: "a.jpg" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaWindow(5, 3)).resolves.toEqual(items);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_window", {
      mediaId: 5,
      radius: 3,
    });
  });
});

describe("folder scanning", () => {
  it("scanFolder invokes with folderId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await scanFolder(7);
    expect(mockInvoke).toHaveBeenCalledWith("scan_folder", { folderId: 7 });
  });

  it("onScanProgress listens on scan-progress and forwards payload", async () => {
    let eventCallback: (event: { payload: unknown }) => void = () => {};
    mockListen.mockImplementation((_event, callback) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();
    await onScanProgress(handler);
    expect(mockListen).toHaveBeenCalledWith(
      "scan-progress",
      expect.any(Function),
    );
    const progress = {
      folder_id: 1,
      scanned: 5,
      total: 10,
      errors: 0,
      status: "scanning",
    };
    eventCallback({ payload: progress });
    expect(handler).toHaveBeenCalledWith(progress);
  });

  it("onFolderChanged listens on folder-changed and forwards folder_id", async () => {
    let eventCallback: (event: {
      payload: { folder_id: number };
    }) => void = () => {};
    mockListen.mockImplementation((_event, callback) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();
    await onFolderChanged(handler);
    expect(mockListen).toHaveBeenCalledWith(
      "folder-changed",
      expect.any(Function),
    );
    eventCallback({ payload: { folder_id: 42 } });
    expect(handler).toHaveBeenCalledWith(42);
  });
});

describe("deduplication", () => {
  it("runDedupScan invokes run_dedup_scan", async () => {
    const result = {
      exact_groups: 1,
      perceptual_groups: 2,
      total_duplicates: 5,
    };
    mockInvoke.mockResolvedValue(result);
    await expect(runDedupScan()).resolves.toEqual(result);
    expect(mockInvoke).toHaveBeenCalledWith("run_dedup_scan");
  });

  it("getDuplicateGroups invokes get_duplicate_groups", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(getDuplicateGroups()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("get_duplicate_groups");
  });

  it("getDuplicateCount invokes get_duplicate_count", async () => {
    mockInvoke.mockResolvedValue(3);
    await expect(getDuplicateCount()).resolves.toBe(3);
    expect(mockInvoke).toHaveBeenCalledWith("get_duplicate_count");
  });

  it("resolveDuplicate invokes with groupId, keepMediaId, and deleteFiles", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await resolveDuplicate(1, 10, true);
    expect(mockInvoke).toHaveBeenCalledWith("resolve_duplicate", {
      groupId: 1,
      keepMediaId: 10,
      deleteFiles: true,
    });
  });

  it("dismissDuplicateGroup invokes with groupId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await dismissDuplicateGroup(2);
    expect(mockInvoke).toHaveBeenCalledWith("dismiss_duplicate_group", {
      groupId: 2,
    });
  });
});

describe("location", () => {
  it("getLocationGroups invokes get_location_groups", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(getLocationGroups()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("get_location_groups");
  });

  it("getMediaByLocation invokes with country, city, offset, and limit", async () => {
    const items = [{ id: 1, filename: "geo.jpg" }];
    mockInvoke.mockResolvedValue(items);
    await expect(getMediaByLocation("CN", "Beijing", 0, 50)).resolves.toEqual(
      items,
    );
    expect(mockInvoke).toHaveBeenCalledWith("get_media_by_location", {
      country: "CN",
      city: "Beijing",
      offset: 0,
      limit: 50,
    });
  });

  it("getLocationStats invokes get_location_stats", async () => {
    const stats = { total_with_gps: 100, countries: 5, cities: 20 };
    mockInvoke.mockResolvedValue(stats);
    await expect(getLocationStats()).resolves.toEqual(stats);
    expect(mockInvoke).toHaveBeenCalledWith("get_location_stats");
  });
});

describe("albums", () => {
  it("createAlbum invokes with name and null description by default", async () => {
    const album = {
      id: 1,
      name: "Trip",
      description: null,
      cover_media_id: null,
      media_count: 0,
      created_at: "",
      updated_at: "",
    };
    mockInvoke.mockResolvedValue(album);
    await expect(createAlbum("Trip")).resolves.toEqual(album);
    expect(mockInvoke).toHaveBeenCalledWith("create_album", {
      name: "Trip",
      description: null,
    });
  });

  it("createAlbum passes description when provided", async () => {
    mockInvoke.mockResolvedValue({});
    await createAlbum("Trip", "Summer vacation");
    expect(mockInvoke).toHaveBeenCalledWith("create_album", {
      name: "Trip",
      description: "Summer vacation",
    });
  });

  it("deleteAlbum invokes with id", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteAlbum(1);
    expect(mockInvoke).toHaveBeenCalledWith("delete_album", { id: 1 });
  });

  it("updateAlbum invokes with id, name, and description", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await updateAlbum(1, "Renamed", "New desc");
    expect(mockInvoke).toHaveBeenCalledWith("update_album", {
      id: 1,
      name: "Renamed",
      description: "New desc",
    });
  });

  it("setAlbumCover invokes with albumId and mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await setAlbumCover(1, 10);
    expect(mockInvoke).toHaveBeenCalledWith("set_album_cover", {
      albumId: 1,
      mediaId: 10,
    });
  });

  it("listAlbums invokes list_albums", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listAlbums()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_albums");
  });

  it("addToAlbum invokes with albumId and mediaIds", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await addToAlbum(1, [2, 3]);
    expect(mockInvoke).toHaveBeenCalledWith("add_to_album", {
      albumId: 1,
      mediaIds: [2, 3],
    });
  });

  it("removeFromAlbum invokes with albumId and mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await removeFromAlbum(1, 5);
    expect(mockInvoke).toHaveBeenCalledWith("remove_from_album", {
      albumId: 1,
      mediaId: 5,
    });
  });

  it("getAlbumMedia invokes with albumId, offset, and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getAlbumMedia(1, 0, 60);
    expect(mockInvoke).toHaveBeenCalledWith("get_album_media", {
      albumId: 1,
      offset: 0,
      limit: 60,
    });
  });
});

describe("favorites", () => {
  it("getFavorites invokes with offset and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getFavorites(10, 50);
    expect(mockInvoke).toHaveBeenCalledWith("get_favorites", {
      offset: 10,
      limit: 50,
    });
  });

  it("getFavoritesCount invokes get_favorites_count", async () => {
    mockInvoke.mockResolvedValue(7);
    await expect(getFavoritesCount()).resolves.toBe(7);
    expect(mockInvoke).toHaveBeenCalledWith("get_favorites_count");
  });
});

describe("trash", () => {
  it("deleteMedia invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteMedia(5);
    expect(mockInvoke).toHaveBeenCalledWith("delete_media", { mediaId: 5 });
  });

  it("getDeletedMedia invokes get_deleted_media", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(getDeletedMedia()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("get_deleted_media");
  });

  it("restoreMedia invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await restoreMedia(5);
    expect(mockInvoke).toHaveBeenCalledWith("restore_media", { mediaId: 5 });
  });

  it("permanentlyDelete invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await permanentlyDelete(5);
    expect(mockInvoke).toHaveBeenCalledWith("permanently_delete", {
      mediaId: 5,
    });
  });

  it("batchDeleteMedia invokes with mediaIds", async () => {
    mockInvoke.mockResolvedValue(2);
    await expect(batchDeleteMedia([1, 2])).resolves.toBe(2);
    expect(mockInvoke).toHaveBeenCalledWith("batch_delete_media", {
      mediaIds: [1, 2],
    });
  });

  it("batchAddToAlbum invokes with albumId and mediaIds", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await batchAddToAlbum(1, [2, 3]);
    expect(mockInvoke).toHaveBeenCalledWith("batch_add_to_album", {
      albumId: 1,
      mediaIds: [2, 3],
    });
  });

  it("batchToggleFavorite invokes with mediaIds and favorite", async () => {
    mockInvoke.mockResolvedValue(3);
    await expect(batchToggleFavorite([1, 2, 3], true)).resolves.toBe(3);
    expect(mockInvoke).toHaveBeenCalledWith("batch_toggle_favorite", {
      mediaIds: [1, 2, 3],
      favorite: true,
    });
  });

  it("batchRestoreMedia invokes with mediaIds", async () => {
    mockInvoke.mockResolvedValue(2);
    await expect(batchRestoreMedia([1, 2])).resolves.toBe(2);
    expect(mockInvoke).toHaveBeenCalledWith("batch_restore_media", {
      mediaIds: [1, 2],
    });
  });

  it("batchPermanentDelete invokes with mediaIds", async () => {
    mockInvoke.mockResolvedValue(2);
    await expect(batchPermanentDelete([1, 2])).resolves.toBe(2);
    expect(mockInvoke).toHaveBeenCalledWith("batch_permanent_delete", {
      mediaIds: [1, 2],
    });
  });
});

describe("search", () => {
  it("searchMedia invokes with query, limit, and offset", async () => {
    mockInvoke.mockResolvedValue([]);
    await searchMedia("sunset", 20, 0);
    expect(mockInvoke).toHaveBeenCalledWith("search_media", {
      query: "sunset",
      limit: 20,
      offset: 0,
    });
  });

  it("searchMediaCount invokes with query", async () => {
    mockInvoke.mockResolvedValue(5);
    await expect(searchMediaCount("sunset")).resolves.toBe(5);
    expect(mockInvoke).toHaveBeenCalledWith("search_media_count", {
      query: "sunset",
    });
  });

  it("semanticSearch invokes with default limit of 50", async () => {
    const response = { results: [], used_semantic: true };
    mockInvoke.mockResolvedValue(response);
    await expect(semanticSearch("beach")).resolves.toEqual(response);
    expect(mockInvoke).toHaveBeenCalledWith("semantic_search", {
      queryText: "beach",
      limit: 50,
    });
  });

  it("semanticSearch passes custom limit", async () => {
    mockInvoke.mockResolvedValue({ results: [], used_semantic: false });
    await semanticSearch("beach", 10);
    expect(mockInvoke).toHaveBeenCalledWith("semantic_search", {
      queryText: "beach",
      limit: 10,
    });
  });
});

describe("smart albums", () => {
  it("createSmartAlbum invokes with name, icon, and rule", async () => {
    const rule = { media_type: "Photo", is_favorite: true };
    mockInvoke.mockResolvedValue({
      id: 1,
      name: "Favorites",
      icon: "star",
      rule_json: "{}",
      media_count: 0,
      created_at: "",
    });
    await createSmartAlbum("Favorites", "star", rule);
    expect(mockInvoke).toHaveBeenCalledWith("create_smart_album", {
      name: "Favorites",
      icon: "star",
      rule,
    });
  });

  it("listSmartAlbums invokes list_smart_albums", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listSmartAlbums()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_smart_albums");
  });

  it("deleteSmartAlbum invokes with id", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteSmartAlbum(1);
    expect(mockInvoke).toHaveBeenCalledWith("delete_smart_album", { id: 1 });
  });

  it("getSmartAlbumMedia invokes with id, offset, and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getSmartAlbumMedia(1, 0, 60);
    expect(mockInvoke).toHaveBeenCalledWith("get_smart_album_media", {
      id: 1,
      offset: 0,
      limit: 60,
    });
  });
});

describe("memories", () => {
  it("generateMemories invokes generate_memories", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(generateMemories()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("generate_memories");
  });

  it("getOnThisDay invokes with default limit of 20", async () => {
    mockInvoke.mockResolvedValue([]);
    await getOnThisDay();
    expect(mockInvoke).toHaveBeenCalledWith("get_on_this_day", { limit: 20 });
  });

  it("getOnThisDay passes custom limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getOnThisDay(50);
    expect(mockInvoke).toHaveBeenCalledWith("get_on_this_day", { limit: 50 });
  });

  it("listMemories invokes list_memories", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listMemories()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_memories");
  });

  it("getMemoryMedia invokes with memoryId, offset, and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getMemoryMedia(1, 0, 30);
    expect(mockInvoke).toHaveBeenCalledWith("get_memory_media", {
      memoryId: 1,
      offset: 0,
      limit: 30,
    });
  });
});

describe("AI and models", () => {
  it("getAiStatus invokes get_ai_status", async () => {
    const status = {
      python_available: true,
      clip_available: true,
      face_available: false,
      status_message: "ok",
    };
    mockInvoke.mockResolvedValue(status);
    await expect(getAiStatus()).resolves.toEqual(status);
    expect(mockInvoke).toHaveBeenCalledWith("get_ai_status");
  });

  it("getModelStatus invokes get_model_status", async () => {
    mockInvoke.mockResolvedValue({
      models_dir: "/models",
      clip_available: true,
      face_available: false,
      models: [],
    });
    await getModelStatus();
    expect(mockInvoke).toHaveBeenCalledWith("get_model_status");
  });

  it("downloadModel invokes with filename", async () => {
    mockInvoke.mockResolvedValue("/models/clip.onnx");
    await expect(downloadModel("clip.onnx")).resolves.toBe("/models/clip.onnx");
    expect(mockInvoke).toHaveBeenCalledWith("download_model", {
      filename: "clip.onnx",
    });
  });

  it("openModelsDir invokes open_models_dir", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await openModelsDir();
    expect(mockInvoke).toHaveBeenCalledWith("open_models_dir");
  });
});

describe("screenshots", () => {
  it('getScreenshots("all") passes screenshotType null', async () => {
    mockInvoke.mockResolvedValue([]);
    await getScreenshots("all", 0, 60);
    expect(mockInvoke).toHaveBeenCalledWith("get_screenshots", {
      screenshotType: null,
      limit: 60,
      offset: 0,
    });
  });

  it('getScreenshots("code") passes screenshotType "code"', async () => {
    mockInvoke.mockResolvedValue([]);
    await getScreenshots("code", 10, 30);
    expect(mockInvoke).toHaveBeenCalledWith("get_screenshots", {
      screenshotType: "code",
      limit: 30,
      offset: 10,
    });
  });

  it('getScreenshotCount("all") passes screenshotType null', async () => {
    mockInvoke.mockResolvedValue(100);
    await expect(getScreenshotCount("all")).resolves.toBe(100);
    expect(mockInvoke).toHaveBeenCalledWith("get_screenshot_count", {
      screenshotType: null,
    });
  });

  it('getScreenshotCount("code") passes screenshotType "code"', async () => {
    mockInvoke.mockResolvedValue(5);
    await getScreenshotCount("code");
    expect(mockInvoke).toHaveBeenCalledWith("get_screenshot_count", {
      screenshotType: "code",
    });
  });
});

describe("CLIP embeddings", () => {
  it("computeClipEmbedding invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await computeClipEmbedding(42);
    expect(mockInvoke).toHaveBeenCalledWith("compute_clip_embedding", {
      mediaId: 42,
    });
  });

  it("computeClipEmbeddingsBatch invokes with default limit", async () => {
    mockInvoke.mockResolvedValue({
      processed: 10,
      succeeded: 9,
      failed: 1,
      errors: [],
    });
    await computeClipEmbeddingsBatch();
    expect(mockInvoke).toHaveBeenCalledWith("compute_clip_embeddings_batch", {
      limit: 32,
    });
  });

  it("findSimilarPhotos invokes with default limit of 20", async () => {
    mockInvoke.mockResolvedValue([]);
    await findSimilarPhotos(5);
    expect(mockInvoke).toHaveBeenCalledWith("find_similar_photos", {
      mediaId: 5,
      limit: 20,
    });
  });

  it("findSimilarPhotos passes custom limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await findSimilarPhotos(5, 10);
    expect(mockInvoke).toHaveBeenCalledWith("find_similar_photos", {
      mediaId: 5,
      limit: 10,
    });
  });
});

describe("face detection and persons", () => {
  it("detectFaces invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue([]);
    await detectFaces(1);
    expect(mockInvoke).toHaveBeenCalledWith("detect_faces", { mediaId: 1 });
  });

  it("detectFacesBatch invokes detect_faces_batch", async () => {
    mockInvoke.mockResolvedValue({ media_processed: 10, faces_found: 25 });
    await detectFacesBatch();
    expect(mockInvoke).toHaveBeenCalledWith("detect_faces_batch");
  });

  it("onFaceDetectionProgress listens and forwards payload", async () => {
    let eventCallback: (event: { payload: unknown }) => void = () => {};
    mockListen.mockImplementation((_event, callback) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();
    await onFaceDetectionProgress(handler);
    expect(mockListen).toHaveBeenCalledWith(
      "face-detection-progress",
      expect.any(Function),
    );
    const progress = {
      processed: 5,
      total: 10,
      faces_found: 3,
      status: "detecting",
    };
    eventCallback({ payload: progress });
    expect(handler).toHaveBeenCalledWith(progress);
  });

  it("getFaces invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue([]);
    await getFaces(1);
    expect(mockInvoke).toHaveBeenCalledWith("get_faces", { mediaId: 1 });
  });

  it("getPersonFaces invokes with personId, offset, and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getPersonFaces(2, 0, 50);
    expect(mockInvoke).toHaveBeenCalledWith("get_person_faces", {
      personId: 2,
      offset: 0,
      limit: 50,
    });
  });

  it("listPersons invokes list_persons", async () => {
    mockInvoke.mockResolvedValue([]);
    await expect(listPersons()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith("list_persons");
  });

  it("getPersonMedia invokes with personId, offset, and limit", async () => {
    mockInvoke.mockResolvedValue([]);
    await getPersonMedia(2, 0, 60);
    expect(mockInvoke).toHaveBeenCalledWith("get_person_media", {
      personId: 2,
      offset: 0,
      limit: 60,
    });
  });

  it("renamePerson invokes with personId and name", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await renamePerson(2, "Alice");
    expect(mockInvoke).toHaveBeenCalledWith("rename_person", {
      personId: 2,
      name: "Alice",
    });
  });

  it("clusterFaces invokes with default threshold null", async () => {
    mockInvoke.mockResolvedValue([]);
    await clusterFaces();
    expect(mockInvoke).toHaveBeenCalledWith("cluster_faces", {
      threshold: null,
    });
  });

  it("clusterFaces passes custom threshold", async () => {
    mockInvoke.mockResolvedValue([]);
    await clusterFaces(0.6);
    expect(mockInvoke).toHaveBeenCalledWith("cluster_faces", {
      threshold: 0.6,
    });
  });

  it("mergePersons is a no-op with fewer than 2 ids", async () => {
    await mergePersons([]);
    await mergePersons([1]);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("mergePersons loops merge_persons for each pair", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await mergePersons([1, 2, 3]);
    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "merge_persons", {
      personIdA: 1,
      personIdB: 2,
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "merge_persons", {
      personIdA: 1,
      personIdB: 3,
    });
  });

  it("splitFaceFromPerson invokes with faceId and null newPersonName by default", async () => {
    mockInvoke.mockResolvedValue(5);
    await expect(splitFaceFromPerson(10)).resolves.toBe(5);
    expect(mockInvoke).toHaveBeenCalledWith("split_face_from_person", {
      faceId: 10,
      newPersonName: null,
    });
  });

  it("splitFaceFromPerson passes newPersonName when provided", async () => {
    mockInvoke.mockResolvedValue(6);
    await splitFaceFromPerson(10, "Bob");
    expect(mockInvoke).toHaveBeenCalledWith("split_face_from_person", {
      faceId: 10,
      newPersonName: "Bob",
    });
  });
});

describe("edits", () => {
  it("revertEdit invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await revertEdit(1);
    expect(mockInvoke).toHaveBeenCalledWith("revert_edit", { mediaId: 1 });
  });

  it("hasEdits invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(hasEdits(1)).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("has_edits", { mediaId: 1 });
  });

  it("exportEdited invokes with default quality of 92", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await exportEdited(1, "/output/edited.jpg");
    expect(mockInvoke).toHaveBeenCalledWith("export_edited", {
      mediaId: 1,
      outputPath: "/output/edited.jpg",
      quality: 92,
    });
  });

  it("exportEdited passes custom quality", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await exportEdited(1, "/output/edited.jpg", 80);
    expect(mockInvoke).toHaveBeenCalledWith("export_edited", {
      mediaId: 1,
      outputPath: "/output/edited.jpg",
      quality: 80,
    });
  });
});

describe("thumbnail regeneration", () => {
  it("regenerateThumbnails invokes regenerate_thumbnails", async () => {
    mockInvoke.mockResolvedValue({ regenerated: 10 });
    await expect(regenerateThumbnails()).resolves.toEqual({ regenerated: 10 });
    expect(mockInvoke).toHaveBeenCalledWith("regenerate_thumbnails");
  });

  it("regenerateThumbnailSingle invokes with mediaId", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(regenerateThumbnailSingle(5)).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("regenerate_thumbnail_single", {
      mediaId: 5,
    });
  });

  it("onThumbnailRegenProgress listens and forwards payload", async () => {
    let eventCallback: (event: { payload: unknown }) => void = () => {};
    mockListen.mockImplementation((_event, callback) => {
      eventCallback = callback;
      return Promise.resolve(() => {});
    });
    const handler = vi.fn();
    await onThumbnailRegenProgress(handler);
    expect(mockListen).toHaveBeenCalledWith(
      "thumbnail-regen-progress",
      expect.any(Function),
    );
    const progress = {
      processed: 5,
      total: 10,
      regenerated: 4,
      status: "running",
    };
    eventCallback({ payload: progress });
    expect(handler).toHaveBeenCalledWith(progress);
  });
});

describe("geo", () => {
  it("getMediaWithGeo invokes with default limit and offset", async () => {
    mockInvoke.mockResolvedValue([]);
    await getMediaWithGeo();
    expect(mockInvoke).toHaveBeenCalledWith("get_media_with_geo", {
      limit: 5000,
      offset: 0,
    });
  });

  it("getMediaWithGeo passes custom limit and offset", async () => {
    mockInvoke.mockResolvedValue([]);
    await getMediaWithGeo(100, 50);
    expect(mockInvoke).toHaveBeenCalledWith("get_media_with_geo", {
      limit: 100,
      offset: 50,
    });
  });

  it("getGeoClusters invokes with default gridSize", async () => {
    mockInvoke.mockResolvedValue([]);
    await getGeoClusters();
    expect(mockInvoke).toHaveBeenCalledWith("get_geo_clusters", {
      gridSize: 0.5,
    });
  });

  it("getGeoClusters passes custom gridSize", async () => {
    mockInvoke.mockResolvedValue([]);
    await getGeoClusters(1.0);
    expect(mockInvoke).toHaveBeenCalledWith("get_geo_clusters", {
      gridSize: 1.0,
    });
  });
});

describe("invokeCommand error wrapping", () => {
  it("throws Error with localized message when invoke fails", async () => {
    mockInvoke.mockRejectedValue(new Error("media not found"));
    await expect(getMediaCount()).rejects.toThrow("Resource not found");
  });

  it("throws Error with localized message for invokeCommand with args", async () => {
    mockInvoke.mockRejectedValue(new Error("database locked"));
    await expect(toggleFavorite(1)).rejects.toThrow(
      "Database error, please try again",
    );
  });
});
