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
} from "@/store/appStore";
import type { MediaItem, WatchedFolder } from "@/lib/tauri";

function resetStore() {
  setView("all");
  setWatchedFolders([]);
  setMedia([], 0);
  clearMediaSelection();
  setScanning(false, null);
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
});
