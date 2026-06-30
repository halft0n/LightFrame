import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  getSnapshot,
  subscribe,
  setView,
  setWatchedFolders,
  addFolder,
  removeFolder,
  setMedia,
  setScanning,
  toggleMediaSelection,
  clearMediaSelection,
  selectMediaRange,
  openViewer,
  closeViewer,
  setSearchQuery,
  setTheme,
  setThumbnailSize,
  setMediaSelection,
  openAlbumDetail,
  closeAlbumDetail,
  openSmartAlbumDetail,
  closeSmartAlbumDetail,
  openMemoryDetail,
  closeMemoryDetail,
  openPersonDetail,
  closePersonDetail,
  navigate,
  addSearchHistory,
  clearSearchHistory,
  loadMedia,
  loadMoreMedia,
  appendMedia,
  mergeNewMedia,
  startSlideshow,
  closeSlideshow,
  nextSlideshow,
  prevSlideshow,
  setSlideshowIndex,
  setSlideshowSpeed,
  setSearchMode,
  updateFolder,
  setSingleMediaSelection,
  setMediaScrollIndex,
} from "@/store/appStore";
import type { MediaItem, WatchedFolder } from "@/lib/tauri";

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

const getMediaPage = vi.fn();
const getMediaCount = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getMediaPage: (...args: unknown[]) => getMediaPage(...args),
    getMediaCount: (...args: unknown[]) => getMediaCount(...args),
  };
});

function resetStore() {
  setView("all");
  setWatchedFolders([]);
  setMedia([], 0);
  clearMediaSelection();
  setScanning(false, null);
  closeViewer();
  closeSlideshow();
  setSearchQuery("");
  setSearchMode("text");
  setTheme("dark");
  setThumbnailSize("medium");
  closeAlbumDetail();
  closeSmartAlbumDetail();
  closeMemoryDetail();
  closePersonDetail();
  clearSearchHistory();
  setView("all");
  getMediaPage.mockReset();
  getMediaCount.mockReset();
}

const sampleFolder: WatchedFolder = {
  id: 1,
  path: "/photos",
  media_count: 0,
  scan_status: "idle",
};

const sampleMedia: MediaItem = {
  id: 1,
  path: "/photos/sunset.jpg",
  filename: "sunset.jpg",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-01-01T00:00:00",
};

beforeEach(() => {
  resetStore();
});

describe("appStore", () => {
  it("has correct initial state values", () => {
    const state = getSnapshot();
    expect(state.currentView).toBe("all");
    expect(state.selectedMediaIds).toEqual([]);
    expect(state.watchedFolders).toEqual([]);
    expect(state.mediaItems).toEqual([]);
    expect(state.totalCount).toBe(0);
    expect(state.mediaCursor).toBeNull();
    expect(state.isScanning).toBe(false);
    expect(state.scanProgress).toBeNull();
  });

  it("setView changes currentView", () => {
    setView("timeline");
    expect(getSnapshot().currentView).toBe("timeline");

    setView("settings");
    expect(getSnapshot().currentView).toBe("settings");
  });

  it("setMedia updates mediaItems and totalCount", () => {
    const items = [
      sampleMedia,
      { ...sampleMedia, id: 2, filename: "beach.jpg" },
    ];
    setMedia(items, 42);

    const state = getSnapshot();
    expect(state.mediaItems).toEqual(items);
    expect(state.totalCount).toBe(42);
  });

  it("setWatchedFolders replaces folder list", () => {
    addFolder(sampleFolder);
    const folders = [sampleFolder, { ...sampleFolder, id: 2, path: "/videos" }];
    setWatchedFolders(folders);
    expect(getSnapshot().watchedFolders).toEqual(folders);
  });

  it("addFolder and removeFolder manage folder list", () => {
    addFolder(sampleFolder);
    addFolder({ ...sampleFolder, id: 2, path: "/videos" });

    expect(getSnapshot().watchedFolders).toHaveLength(2);

    removeFolder(1);
    expect(getSnapshot().watchedFolders).toHaveLength(1);
    expect(getSnapshot().watchedFolders[0].id).toBe(2);
  });

  it("toggleMediaSelection and clearMediaSelection", () => {
    toggleMediaSelection(1);
    expect(getSnapshot().selectedMediaIds).toEqual([1]);

    toggleMediaSelection(2);
    expect(getSnapshot().selectedMediaIds).toEqual([1, 2]);

    toggleMediaSelection(1);
    expect(getSnapshot().selectedMediaIds).toEqual([2]);

    clearMediaSelection();
    expect(getSnapshot().selectedMediaIds).toEqual([]);
  });

  it("setScanning updates state", () => {
    const progress = {
      folder_id: 1,
      scanned: 5,
      total: 10,
      errors: 0,
      status: "scanning" as const,
    };
    setScanning(true, progress);

    const state = getSnapshot();
    expect(state.isScanning).toBe(true);
    expect(state.scanProgress).toEqual(progress);

    setScanning(false);
    expect(getSnapshot().isScanning).toBe(false);
    expect(getSnapshot().scanProgress).toBeNull();
  });

  it("subscribe notifies listeners", () => {
    const listener = vi.fn();
    const unsubscribe = subscribe(listener);

    setView("duplicates");
    expect(listener).toHaveBeenCalledTimes(1);

    unsubscribe();
    setView("all");
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("selectMediaRange selects inclusive range from store mediaItems", () => {
    const items = [
      sampleMedia,
      { ...sampleMedia, id: 2 },
      { ...sampleMedia, id: 3 },
      { ...sampleMedia, id: 4 },
    ];
    setMedia(items, 4);

    selectMediaRange(1, 3);
    expect(getSnapshot().selectedMediaIds).toEqual([1, 2, 3]);

    selectMediaRange(4, 2);
    expect(getSnapshot().selectedMediaIds).toEqual([2, 3, 4]);
  });

  it("selectMediaRange uses contextItems when provided", () => {
    setMedia([sampleMedia, { ...sampleMedia, id: 2 }], 2);
    const contextItems = [{ id: 10 }, { id: 20 }, { id: 30 }, { id: 40 }];

    selectMediaRange(10, 30, contextItems);
    expect(getSnapshot().selectedMediaIds).toEqual([10, 20, 30]);
  });

  it("selectMediaRange does nothing when IDs are not found", () => {
    setMedia([sampleMedia, { ...sampleMedia, id: 2 }], 2);
    toggleMediaSelection(1);

    selectMediaRange(1, 99);
    expect(getSnapshot().selectedMediaIds).toEqual([1]);

    selectMediaRange(99, 1, [{ id: 10 }, { id: 20 }]);
    expect(getSnapshot().selectedMediaIds).toEqual([1]);
  });

  it("openViewer and closeViewer manage viewingMediaId", () => {
    expect(getSnapshot().viewingMediaId).toBeNull();

    openViewer(42);
    expect(getSnapshot().viewingMediaId).toBe(42);

    closeViewer();
    expect(getSnapshot().viewingMediaId).toBeNull();
  });

  it("setSearchQuery updates searchQuery", () => {
    setSearchQuery("sunset beach");
    expect(getSnapshot().searchQuery).toBe("sunset beach");

    setSearchQuery("");
    expect(getSnapshot().searchQuery).toBe("");
  });

  it("setTheme updates theme", () => {
    setTheme("light");
    expect(getSnapshot().theme).toBe("light");

    setTheme("system");
    expect(getSnapshot().theme).toBe("system");

    setTheme("dark");
    expect(getSnapshot().theme).toBe("dark");
  });

  it("setThumbnailSize persists thumbnail size preference", () => {
    expect(getSnapshot().thumbnailSize).toBe("medium");

    setThumbnailSize("small");
    expect(getSnapshot().thumbnailSize).toBe("small");

    setThumbnailSize("large");
    expect(getSnapshot().thumbnailSize).toBe("large");
  });

  it("navigates to all primary views", () => {
    const views = [
      "all",
      "videos",
      "timeline",
      "favorites",
      "people",
      "duplicates",
      "screenshots",
      "albums",
      "smart-albums",
      "memories",
      "deleted",
      "settings",
      "locations",
    ] as const;

    for (const view of views) {
      setView(view);
      expect(getSnapshot().currentView).toBe(view);
    }
  });

  it("select all and clear selection edge cases", () => {
    const items = [
      sampleMedia,
      { ...sampleMedia, id: 2 },
      { ...sampleMedia, id: 3 },
    ];
    setMedia(items, 3);

    setMediaSelection(items.map((m) => m.id));
    expect(getSnapshot().selectedMediaIds).toEqual([1, 2, 3]);

    clearMediaSelection();
    expect(getSnapshot().selectedMediaIds).toEqual([]);

    setMediaSelection([]);
    expect(getSnapshot().selectedMediaIds).toEqual([]);
  });

  it("album detail navigation", () => {
    openAlbumDetail(5);
    expect(getSnapshot().currentView).toBe("album-detail");
    expect(getSnapshot().selectedAlbumId).toBe(5);

    closeAlbumDetail();
    expect(getSnapshot().currentView).toBe("albums");
    expect(getSnapshot().selectedAlbumId).toBeNull();
  });

  it("smart album detail navigation", () => {
    openSmartAlbumDetail(7);
    expect(getSnapshot().currentView).toBe("smart-album-detail");
    expect(getSnapshot().selectedSmartAlbumId).toBe(7);

    closeSmartAlbumDetail();
    expect(getSnapshot().currentView).toBe("smart-albums");
    expect(getSnapshot().selectedSmartAlbumId).toBeNull();
  });

  it("memory detail navigation", () => {
    openMemoryDetail(3);
    expect(getSnapshot().currentView).toBe("memory-detail");
    expect(getSnapshot().selectedMemoryId).toBe(3);

    closeMemoryDetail();
    expect(getSnapshot().currentView).toBe("memories");
    expect(getSnapshot().selectedMemoryId).toBeNull();
  });

  it("person detail navigation", () => {
    openPersonDetail(9);
    expect(getSnapshot().currentView).toBe("person-detail");
    expect(getSnapshot().selectedPersonId).toBe(9);

    closePersonDetail();
    expect(getSnapshot().currentView).toBe("people");
    expect(getSnapshot().selectedPersonId).toBeNull();
  });

  it("setView clears detail IDs when leaving detail views", () => {
    openAlbumDetail(1);
    setView("all");
    expect(getSnapshot().selectedAlbumId).toBeNull();

    openSmartAlbumDetail(2);
    setView("timeline");
    expect(getSnapshot().selectedSmartAlbumId).toBeNull();

    openMemoryDetail(3);
    setView("favorites");
    expect(getSnapshot().selectedMemoryId).toBeNull();

    openPersonDetail(4);
    setView("settings");
    expect(getSnapshot().selectedPersonId).toBeNull();
  });

  it("setView preserves detail IDs when staying on same detail view", () => {
    openAlbumDetail(5);
    setView("album-detail");
    expect(getSnapshot().selectedAlbumId).toBe(5);
  });

  it("navigate to folder sets folder state and clears detail IDs", () => {
    openAlbumDetail(3);
    navigate("folder", { folderId: 7, folderPath: "/photos/vacation" });

    const state = getSnapshot();
    expect(state.currentView).toBe("folder");
    expect(state.selectedFolderId).toBe(7);
    expect(state.selectedFolderPath).toBe("/photos/vacation");
    expect(state.selectedAlbumId).toBeNull();
  });

  it("navigate to folder without params clears folder selection", () => {
    navigate("folder", { folderId: 1, folderPath: "/photos" });
    navigate("folder");

    const state = getSnapshot();
    expect(state.currentView).toBe("folder");
    expect(state.selectedFolderId).toBeNull();
    expect(state.selectedFolderPath).toBeNull();
  });

  it("setView clears folder state when leaving folder view", () => {
    navigate("folder", { folderId: 2, folderPath: "/backup" });
    setView("all");

    const state = getSnapshot();
    expect(state.currentView).toBe("all");
    expect(state.selectedFolderId).toBeNull();
    expect(state.selectedFolderPath).toBeNull();
  });

  it("navigate to map and locations sets view and clears detail IDs", () => {
    openPersonDetail(9);
    navigate("map");

    let state = getSnapshot();
    expect(state.currentView).toBe("map");
    expect(state.selectedPersonId).toBeNull();

    openAlbumDetail(3);
    navigate("locations");
    state = getSnapshot();
    expect(state.currentView).toBe("locations");
    expect(state.selectedAlbumId).toBeNull();
  });

  it("navigate to person-detail preserves selectedPersonId", () => {
    openPersonDetail(12);
    navigate("person-detail");

    const state = getSnapshot();
    expect(state.currentView).toBe("person-detail");
    expect(state.selectedPersonId).toBe(12);
  });

  it("navigate to detail views clears unrelated detail IDs", () => {
    openPersonDetail(5);
    openAlbumDetail(8);

    navigate("album-detail");
    expect(getSnapshot().selectedAlbumId).toBe(8);
    expect(getSnapshot().selectedPersonId).toBeNull();

    openSmartAlbumDetail(4);
    navigate("smart-album-detail");
    expect(getSnapshot().selectedSmartAlbumId).toBe(4);
    expect(getSnapshot().selectedAlbumId).toBeNull();
  });

  describe("search history", () => {
    it("addSearchHistory adds trimmed queries", () => {
      addSearchHistory("  sunset  ");
      expect(getSnapshot().searchHistory).toEqual(["sunset"]);
    });

    it("addSearchHistory ignores empty or whitespace-only queries", () => {
      addSearchHistory("");
      addSearchHistory("   ");
      expect(getSnapshot().searchHistory).toEqual([]);
    });

    it("addSearchHistory moves duplicates to the front", () => {
      addSearchHistory("beach");
      addSearchHistory("sunset");
      addSearchHistory("beach");

      expect(getSnapshot().searchHistory).toEqual(["beach", "sunset"]);
    });

    it("addSearchHistory keeps at most 10 items", () => {
      for (let i = 1; i <= 12; i++) {
        addSearchHistory(`query-${i}`);
      }
      const history = getSnapshot().searchHistory;
      expect(history).toHaveLength(10);
      expect(history[0]).toBe("query-12");
      expect(history[9]).toBe("query-3");
    });

    it("clearSearchHistory removes all entries", () => {
      addSearchHistory("sunset");
      addSearchHistory("beach");
      clearSearchHistory();
      expect(getSnapshot().searchHistory).toEqual([]);
    });
  });

  describe("media loading", () => {
    it("loadMedia fetches page and count", async () => {
      const items = [sampleMedia, { ...sampleMedia, id: 2 }];
      getMediaPage.mockResolvedValue(items);
      getMediaCount.mockResolvedValue(100);

      await loadMedia();

      const state = getSnapshot();
      expect(state.mediaItems).toEqual(items);
      expect(state.totalCount).toBe(100);
      expect(getMediaPage).toHaveBeenCalledWith(60);
      expect(getMediaCount).toHaveBeenCalled();
    });

    it("loadMedia handles errors without throwing", async () => {
      getMediaPage.mockRejectedValue(new Error("network"));
      getMediaCount.mockRejectedValue(new Error("network"));
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      await loadMedia();

      expect(getSnapshot().mediaItems).toEqual([]);
      consoleSpy.mockRestore();
    });

    it("appendMedia adds items to existing media", () => {
      setMedia([sampleMedia], 10);
      appendMedia([
        { ...sampleMedia, id: 2 },
        { ...sampleMedia, id: 3 },
      ]);

      expect(getSnapshot().mediaItems).toHaveLength(3);
      expect(getSnapshot().mediaItems.map((m) => m.id)).toEqual([1, 2, 3]);
    });

    it("appendMedia does nothing for empty array", () => {
      setMedia([sampleMedia], 1);
      appendMedia([]);
      expect(getSnapshot().mediaItems).toHaveLength(1);
    });

    it("loadMoreMedia appends next page when more items exist", async () => {
      const itemWithCursor = {
        ...sampleMedia,
        created_at: "2024-01-01T00:00:00",
      };
      setMedia([itemWithCursor], 5);
      getMediaPage.mockResolvedValue([
        { ...sampleMedia, id: 2 },
        { ...sampleMedia, id: 3 },
      ]);

      await loadMoreMedia();

      expect(getMediaPage).toHaveBeenCalledWith(60, ["2024-01-01T00:00:00", 1]);
      expect(getSnapshot().mediaItems).toHaveLength(3);
    });

    it("loadMoreMedia skips when all items loaded", async () => {
      setMedia([sampleMedia], 1);
      await loadMoreMedia();
      expect(getMediaPage).not.toHaveBeenCalled();
    });

    it("loadMoreMedia propagates errors after logging", async () => {
      const itemWithCursor = {
        ...sampleMedia,
        created_at: "2024-01-01T00:00:00",
      };
      setMedia([itemWithCursor], 10);
      getMediaPage.mockRejectedValue(new Error("network"));
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      await expect(loadMoreMedia()).rejects.toThrow("network");

      expect(getSnapshot().mediaItems).toHaveLength(1);
      expect(consoleSpy).toHaveBeenCalledWith(
        "Failed to load more media:",
        expect.any(Error),
      );
      consoleSpy.mockRestore();
    });

    it("loadMoreMedia uses undefined cursor when created_at is null", async () => {
      setMedia([sampleMedia], 10);
      getMediaPage.mockResolvedValue([{ ...sampleMedia, id: 2 }]);

      await loadMoreMedia();

      expect(getMediaPage).toHaveBeenCalledWith(60, undefined);
    });

    it("loadMedia sets mediaLoadError on failure", async () => {
      getMediaPage.mockRejectedValue(new Error("connection refused"));
      getMediaCount.mockRejectedValue(new Error("connection refused"));
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      await loadMedia();

      expect(getSnapshot().mediaLoadError).toBe("connection refused");
      consoleSpy.mockRestore();
    });

    it("loadMedia clears mediaLoadError on success", async () => {
      getMediaPage.mockRejectedValueOnce(new Error("fail"));
      getMediaCount.mockRejectedValueOnce(new Error("fail"));
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      await loadMedia();
      expect(getSnapshot().mediaLoadError).toBe("fail");

      getMediaPage.mockResolvedValue([sampleMedia]);
      getMediaCount.mockResolvedValue(1);
      await loadMedia();
      expect(getSnapshot().mediaLoadError).toBeNull();
      consoleSpy.mockRestore();
    });

    it("loadMoreMedia in-flight guard prevents concurrent calls", async () => {
      const itemWithCursor = {
        ...sampleMedia,
        created_at: "2024-01-01T00:00:00",
      };
      setMedia([itemWithCursor], 100);

      let resolveFirst: ((v: MediaItem[]) => void) | undefined;
      getMediaPage.mockImplementationOnce(
        () =>
          new Promise<MediaItem[]>((resolve) => {
            resolveFirst = resolve;
          }),
      );

      const p1 = loadMoreMedia();
      const p2 = loadMoreMedia();

      expect(getMediaPage).toHaveBeenCalledTimes(1);

      resolveFirst!([{ ...sampleMedia, id: 2 }]);
      await p1;
      await p2;

      expect(getSnapshot().mediaItems).toHaveLength(2);
    });

    it("setMedia trims items when count exceeds 5000", () => {
      const items = Array.from({ length: 5100 }, (_, i) => ({
        ...sampleMedia,
        id: i + 1,
      }));
      setMediaScrollIndex(4000);
      setMedia(items, 5100);

      const state = getSnapshot();
      expect(state.mediaItems).toHaveLength(5000);
      expect(state.mediaItems[0].id).toBe(101);
    });

    it("setMedia sets mediaCursor to null when created_at is missing", () => {
      setMedia([sampleMedia], 1);
      expect(getSnapshot().mediaCursor).toBeNull();
    });
  });

  describe("mergeNewMedia", () => {
    it("mergeNewMedia does not modify state when items is empty", () => {
      const items = [
        { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
        { ...sampleMedia, id: 2, created_at: "2024-01-02T00:00:00" },
      ];
      setMedia(items, 2);
      const before = getSnapshot();

      mergeNewMedia([]);

      const after = getSnapshot();
      expect(after.mediaItems).toEqual(before.mediaItems);
      expect(after.totalCount).toBe(before.totalCount);
      expect(after.mediaCursor).toEqual(before.mediaCursor);
    });

    it("mergeNewMedia does not modify state when all items are duplicates", () => {
      const items = [
        { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
        { ...sampleMedia, id: 2, created_at: "2024-01-02T00:00:00" },
      ];
      setMedia(items, 2);
      const before = getSnapshot();

      mergeNewMedia([
        { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
        { ...sampleMedia, id: 2, created_at: "2024-01-02T00:00:00" },
      ]);

      const after = getSnapshot();
      expect(after.mediaItems).toEqual(before.mediaItems);
      expect(after.totalCount).toBe(before.totalCount);
      expect(after.mediaCursor).toEqual(before.mediaCursor);
    });

    it("mergeNewMedia appends new items and sorts by date descending", () => {
      setMedia(
        [
          { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
          { ...sampleMedia, id: 2, created_at: "2024-01-03T00:00:00" },
        ],
        2,
      );

      mergeNewMedia([
        { ...sampleMedia, id: 3, created_at: "2024-01-02T00:00:00" },
      ]);

      const dates = getSnapshot().mediaItems.map(
        (m) => m.created_at ?? m.modified_at,
      );
      expect(dates).toEqual([
        "2024-01-03T00:00:00",
        "2024-01-02T00:00:00",
        "2024-01-01T00:00:00",
      ]);
    });

    it("mergeNewMedia handles items without created_at, falls back to modified_at", () => {
      setMedia(
        [
          {
            ...sampleMedia,
            id: 1,
            created_at: null,
            modified_at: "2024-01-01T00:00:00",
          },
          {
            ...sampleMedia,
            id: 2,
            created_at: null,
            modified_at: "2024-01-03T00:00:00",
          },
        ],
        2,
      );

      mergeNewMedia([
        {
          ...sampleMedia,
          id: 3,
          created_at: null,
          modified_at: "2024-01-02T00:00:00",
        },
      ]);

      const dates = getSnapshot().mediaItems.map(
        (m) => m.created_at ?? m.modified_at,
      );
      expect(dates).toEqual([
        "2024-01-03T00:00:00",
        "2024-01-02T00:00:00",
        "2024-01-01T00:00:00",
      ]);
    });

    it("mergeNewMedia updates totalCount correctly", () => {
      setMedia([sampleMedia], 10);

      mergeNewMedia([
        { ...sampleMedia, id: 2 },
        { ...sampleMedia, id: 3 },
        { ...sampleMedia, id: 4 },
      ]);

      expect(getSnapshot().totalCount).toBe(13);
    });

    it("mergeNewMedia filters duplicates from mixed batch", () => {
      setMedia(
        [
          { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
          { ...sampleMedia, id: 2, created_at: "2024-01-02T00:00:00" },
        ],
        2,
      );

      mergeNewMedia([
        { ...sampleMedia, id: 2, created_at: "2024-01-02T00:00:00" },
        { ...sampleMedia, id: 3, created_at: "2024-01-03T00:00:00" },
        { ...sampleMedia, id: 4, created_at: "2024-01-04T00:00:00" },
      ]);

      const state = getSnapshot();
      expect(state.mediaItems.map((m) => m.id)).toEqual([4, 3, 2, 1]);
      expect(state.totalCount).toBe(4);
    });

    it("mergeNewMedia handles multiple rapid calls", () => {
      setMedia([], 0);

      mergeNewMedia([
        { ...sampleMedia, id: 1, created_at: "2024-01-01T00:00:00" },
      ]);
      mergeNewMedia([
        { ...sampleMedia, id: 2, created_at: "2024-01-03T00:00:00" },
      ]);
      mergeNewMedia([
        { ...sampleMedia, id: 3, created_at: "2024-01-02T00:00:00" },
      ]);

      const state = getSnapshot();
      expect(state.mediaItems.map((m) => m.id)).toEqual([2, 3, 1]);
      expect(state.totalCount).toBe(3);
    });

    it("mergeNewMedia sorts by id descending when dates are equal", () => {
      setMedia(
        [
          {
            ...sampleMedia,
            id: 1,
            created_at: "2024-01-01T00:00:00",
          },
          {
            ...sampleMedia,
            id: 3,
            created_at: "2024-01-01T00:00:00",
          },
        ],
        2,
      );

      mergeNewMedia([
        {
          ...sampleMedia,
          id: 2,
          created_at: "2024-01-01T00:00:00",
        },
      ]);

      expect(getSnapshot().mediaItems.map((m) => m.id)).toEqual([3, 2, 1]);
    });
  });

  describe("slideshow", () => {
    it("startSlideshow activates with media ids", () => {
      openViewer(5);
      startSlideshow([10, 20, 30], 20);

      const state = getSnapshot();
      expect(state.slideshowActive).toBe(true);
      expect(state.slideshowMediaIds).toEqual([10, 20, 30]);
      expect(state.slideshowIndex).toBe(1);
      expect(state.viewingMediaId).toBeNull();
    });

    it("startSlideshow ignores empty media ids", () => {
      startSlideshow([]);
      expect(getSnapshot().slideshowActive).toBe(false);
    });

    it("closeSlideshow resets slideshow state", () => {
      startSlideshow([1, 2]);
      closeSlideshow();

      const state = getSnapshot();
      expect(state.slideshowActive).toBe(false);
      expect(state.slideshowMediaIds).toEqual([]);
      expect(state.slideshowIndex).toBe(0);
    });

    it("nextSlideshow wraps to start", () => {
      startSlideshow([1, 2, 3]);
      expect(getSnapshot().slideshowIndex).toBe(0);

      nextSlideshow();
      expect(getSnapshot().slideshowIndex).toBe(1);

      nextSlideshow();
      nextSlideshow();
      expect(getSnapshot().slideshowIndex).toBe(0);
    });

    it("prevSlideshow wraps to end", () => {
      startSlideshow([1, 2, 3]);

      prevSlideshow();
      expect(getSnapshot().slideshowIndex).toBe(2);

      prevSlideshow();
      expect(getSnapshot().slideshowIndex).toBe(1);
    });

    it("setSlideshowIndex sets index and clamps to valid range", () => {
      startSlideshow([10, 20, 30]);

      setSlideshowIndex(2);
      expect(getSnapshot().slideshowIndex).toBe(2);

      setSlideshowIndex(99);
      expect(getSnapshot().slideshowIndex).toBe(2);

      setSlideshowIndex(-5);
      expect(getSnapshot().slideshowIndex).toBe(0);
    });

    it("setSlideshowIndex does nothing when slideshow is inactive", () => {
      setSlideshowIndex(5);
      expect(getSnapshot().slideshowIndex).toBe(0);
    });

    it("setSlideshowSpeed stores speed value", () => {
      setSlideshowSpeed(3);
      expect(getSnapshot().slideshowSpeed).toBe(3);

      setSlideshowSpeed(10);
      expect(getSnapshot().slideshowSpeed).toBe(10);
    });
  });

  describe("search and folder helpers", () => {
    it("setSearchMode updates search mode", () => {
      setSearchMode("semantic");
      expect(getSnapshot().searchMode).toBe("semantic");

      setSearchMode("text");
      expect(getSnapshot().searchMode).toBe("text");
    });

    it("updateFolder merges partial updates", () => {
      addFolder(sampleFolder);
      updateFolder(1, { media_count: 99, scan_status: "scanning" });

      const folder = getSnapshot().watchedFolders[0];
      expect(folder.media_count).toBe(99);
      expect(folder.scan_status).toBe("scanning");
      expect(folder.path).toBe("/photos");
    });

    it("setSingleMediaSelection replaces selection", () => {
      toggleMediaSelection(1);
      toggleMediaSelection(2);
      setSingleMediaSelection(5);
      expect(getSnapshot().selectedMediaIds).toEqual([5]);
    });

    it("setMediaScrollIndex updates scroll index", () => {
      const items = Array.from({ length: 10 }, (_, i) => ({
        ...sampleMedia,
        id: i + 1,
      }));
      setMedia(items, 10);

      setMediaScrollIndex(5);
      expect(getSnapshot().mediaScrollIndex).toBe(5);
    });

    it("setMediaScrollIndex clamps negative values to zero", () => {
      setMediaScrollIndex(-5);
      expect(getSnapshot().mediaScrollIndex).toBe(0);
    });

    it("updateFolder leaves other folders unchanged", () => {
      addFolder(sampleFolder);
      addFolder({ ...sampleFolder, id: 2, path: "/videos" });
      updateFolder(1, { media_count: 50 });

      const folders = getSnapshot().watchedFolders;
      expect(folders[0].media_count).toBe(50);
      expect(folders[1].media_count).toBe(0);
    });

    it("updateFolder is no-op for unknown folder id", () => {
      addFolder(sampleFolder);
      updateFolder(999, { media_count: 100 });
      expect(getSnapshot().watchedFolders[0].media_count).toBe(0);
    });

    it("removeFolder is no-op for unknown id", () => {
      addFolder(sampleFolder);
      removeFolder(999);
      expect(getSnapshot().watchedFolders).toHaveLength(1);
    });

    it("toggleMediaSelection on same id twice clears selection", () => {
      toggleMediaSelection(7);
      toggleMediaSelection(7);
      expect(getSnapshot().selectedMediaIds).toEqual([]);
    });
  });
});
