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
} from "@/store/appStore";
import type { MediaItem, WatchedFolder } from "@/lib/tauri";

function resetStore() {
  setView("all");
  setWatchedFolders([]);
  setMedia([], 0);
  clearMediaSelection();
  setScanning(false, null);
  closeViewer();
  setSearchQuery("");
  setTheme("dark");
  setThumbnailSize("medium");
  closeAlbumDetail();
  closeSmartAlbumDetail();
  closeMemoryDetail();
  closePersonDetail();
  clearSearchHistory();
  setView("all");
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
    const items = [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }];
    setMedia(items, 42);

    const state = getSnapshot();
    expect(state.mediaItems).toEqual(items);
    expect(state.totalCount).toBe(42);
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
    const progress = { folder_id: 1, scanned: 5, total: 10, status: "scanning" as const };
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
    const contextItems = [
      { id: 10 },
      { id: 20 },
      { id: 30 },
      { id: 40 },
    ];

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
});
