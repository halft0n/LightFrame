import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "@/App";
import { setLocale } from "@/i18n/index";
import {
  clearMediaSelection,
  getSnapshot,
  openViewer,
  setMediaSelection,
  setSearchQuery,
} from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";
import { invoke } from "@tauri-apps/api/core";

const listWatchedFolders = vi.fn();
const onScanProgress = vi.fn();
const onFolderChanged = vi.fn();
const scanFolder = vi.fn();
const getMediaPage = vi.fn();
const getMediaCount = vi.fn();
const searchMedia = vi.fn();
const searchMediaCount = vi.fn();
const batchDeleteMedia = vi.fn();
const batchAddToAlbum = vi.fn();
const listAlbums = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listWatchedFolders: () => listWatchedFolders(),
    onScanProgress: (cb: Parameters<typeof actual.onScanProgress>[0]) => onScanProgress(cb),
    onFolderChanged: (cb: Parameters<typeof actual.onFolderChanged>[0]) => onFolderChanged(cb),
    scanFolder: (...args: Parameters<typeof actual.scanFolder>) => scanFolder(...args),
    getMediaPage: (...args: Parameters<typeof actual.getMediaPage>) => getMediaPage(...args),
    getMediaCount: (...args: Parameters<typeof actual.getMediaCount>) => getMediaCount(...args),
    searchMedia: (...args: Parameters<typeof actual.searchMedia>) => searchMedia(...args),
    searchMediaCount: (...args: Parameters<typeof actual.searchMediaCount>) =>
      searchMediaCount(...args),
    batchDeleteMedia: (...args: Parameters<typeof actual.batchDeleteMedia>) =>
      batchDeleteMedia(...args),
    batchAddToAlbum: (...args: Parameters<typeof actual.batchAddToAlbum>) =>
      batchAddToAlbum(...args),
    listAlbums: (...args: Parameters<typeof actual.listAlbums>) => listAlbums(...args),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
  convertFileSrc: vi.fn((path: string) => `file://${path}`),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

function setupMatchMedia(matches = false) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: query.includes("767px") ? matches : false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

const sampleMedia: MediaItem[] = [
  {
    id: 1,
    path: "/photos/sunset.jpg",
    filename: "sunset.jpg",
    media_type: "Photo",
    size_bytes: 2048,
    modified_at: "2024-01-01T00:00:00",
  },
  {
    id: 2,
    path: "/photos/beach.jpg",
    filename: "beach.jpg",
    media_type: "Photo",
    size_bytes: 2048,
    modified_at: "2024-01-02T00:00:00",
  },
  {
    id: 3,
    path: "/photos/mountain.jpg",
    filename: "mountain.jpg",
    media_type: "Photo",
    size_bytes: 2048,
    modified_at: "2024-01-03T00:00:00",
  },
];

const watchedFolder = {
  id: 1,
  path: "/photos",
  media_count: 3,
  scan_status: "idle" as const,
};

function setupDefaultMocks() {
  listWatchedFolders.mockResolvedValue([watchedFolder]);
  onScanProgress.mockResolvedValue(() => {});
  onFolderChanged.mockResolvedValue(() => {});
  getMediaPage.mockResolvedValue(sampleMedia);
  getMediaCount.mockResolvedValue(sampleMedia.length);
  searchMedia.mockResolvedValue([sampleMedia[0]]);
  searchMediaCount.mockResolvedValue(1);
  batchDeleteMedia.mockResolvedValue(2);
  batchAddToAlbum.mockResolvedValue(undefined);
  listAlbums.mockResolvedValue([
    { id: 10, name: "Trip", media_count: 0, created_at: "2024-01-01" },
  ]);
}

function setupViewerInvoke() {
  (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "get_media_by_id") {
      const id = (args?.id as number | undefined) ?? 1;
      const item = sampleMedia.find((m) => m.id === id) ?? sampleMedia[0];
      return Promise.resolve({ ...item, id });
    }
    if (cmd === "get_media_neighbors") {
      const id = (args?.id as number | undefined) ?? 1;
      const index = sampleMedia.findIndex((m) => m.id === id);
      return Promise.resolve({
        prev_id: index > 0 ? sampleMedia[index - 1].id : null,
        next_id: index >= 0 && index < sampleMedia.length - 1 ? sampleMedia[index + 1].id : null,
      });
    }
    if (cmd === "get_media_window") {
      const mediaId = (args?.mediaId as number | undefined) ?? 1;
      const item = sampleMedia.find((m) => m.id === mediaId) ?? sampleMedia[0];
      return Promise.resolve([{ ...item, id: mediaId }]);
    }
    if (cmd === "has_edits") return Promise.resolve(false);
    if (cmd === "get_edit") return Promise.resolve(null);
    if (cmd === "is_favorite") return Promise.resolve(false);
    if (cmd === "toggle_favorite") return Promise.resolve(true);
    return Promise.resolve(null);
  });
}

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  clearMediaSelection();
  setSearchQuery("");
  setupMatchMedia(false);
  vi.clearAllMocks();
  setupDefaultMocks();
});

describe("Photo browsing workflow", () => {
  it("loads media list on init", async () => {
    render(<App />);

    await waitFor(() => {
      expect(getSnapshot().mediaItems).toHaveLength(3);
      expect(getSnapshot().totalCount).toBe(3);
    });
  });

  it("search filters results", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<App />);

      await waitFor(() => {
        expect(getSnapshot().mediaItems).toHaveLength(3);
      });

      const searchInput = screen.getByPlaceholderText("搜索照片…");
      await user.clear(searchInput);
      await user.type(searchInput, "sunset");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(350);
      });

      await waitFor(() => {
        expect(getSnapshot().searchQuery).toBe("sunset");
      });

      await waitFor(() => {
        expect(searchMedia).toHaveBeenCalled();
      });
    } finally {
      vi.useRealTimers();
      setSearchQuery("");
    }
  });

  it("select and batch delete", async () => {
    const user = userEvent.setup();

    render(<App />);

    await waitFor(() => {
      expect(getSnapshot().mediaItems).toHaveLength(3);
    });

    act(() => {
      setMediaSelection([1, 2]);
    });

    await waitFor(() => {
      expect(screen.getByText(/已选择 2 项/)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "删除" }));
    await user.click(screen.getByText("确定"));

    await waitFor(() => {
      expect(batchDeleteMedia).toHaveBeenCalledWith([1, 2]);
    });
  });
});

describe("Album management workflow", () => {
  it("adds selected photos to an existing album", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(getSnapshot().mediaItems).toHaveLength(3);
    });

    act(() => {
      setMediaSelection([1, 2]);
    });
    await waitFor(() => {
      expect(screen.getByText(/已选择 2 项/)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "加入相簿" }));
    await user.click(screen.getByRole("button", { name: "Trip" }));

    await waitFor(() => {
      expect(batchAddToAlbum).toHaveBeenCalledWith(10, [1, 2]);
    });
  });
});

describe("Search workflow", () => {
  it("updates search query and invokes backend search", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<App />);

      await waitFor(() => {
        expect(getSnapshot().mediaItems).toHaveLength(3);
      });

      const searchInput = screen.getByPlaceholderText("搜索照片…");
      await user.type(searchInput, "beach");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(350);
      });

      await waitFor(() => {
        expect(searchMedia).toHaveBeenCalled();
        expect(getSnapshot().searchQuery).toBe("beach");
      });
    } finally {
      vi.useRealTimers();
      setSearchQuery("");
    }
  });
});

describe("Keyboard navigation workflow", () => {
  it("clears selection on Escape and navigates viewer with arrow keys", async () => {
    setupViewerInvoke();
    render(<App />);

    await waitFor(() => {
      expect(getSnapshot().mediaItems).toHaveLength(3);
    });

    act(() => {
      setMediaSelection([1, 2]);
    });

    await waitFor(() => {
      expect(screen.getByText(/已选择 2 项/)).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => {
      expect(getSnapshot().selectedMediaIds).toHaveLength(0);
    });

    act(() => {
      openViewer(2);
    });

    await waitFor(() => {
      expect(getSnapshot().viewingMediaId).toBe(2);
      expect(invoke).toHaveBeenCalledWith("get_media_neighbors", { id: 2 });
    });

    await waitFor(() => {
      expect(document.querySelector("img.select-none")).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => {
      expect(getSnapshot().viewingMediaId).toBe(1);
    });

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => {
      expect(getSnapshot().viewingMediaId).toBe(3);
    });

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => {
      expect(getSnapshot().viewingMediaId).toBeNull();
    });
  });
});

describe("Error recovery workflow", () => {
  it("retries after failed search load", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      let searchAttempts = 0;
      searchMedia.mockImplementation(() => {
        searchAttempts += 1;
        if (searchAttempts === 1) {
          return Promise.reject(new Error("search failed"));
        }
        return Promise.resolve([sampleMedia[0]]);
      });
      searchMediaCount.mockResolvedValue(1);

      render(<App />);

      await waitFor(() => {
        expect(getSnapshot().mediaItems).toHaveLength(3);
      });

      const searchInput = screen.getByPlaceholderText("搜索照片…");
      await user.type(searchInput, "sunset");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(350);
      });

      await waitFor(() => {
        expect(getSnapshot().searchQuery).toBe("sunset");
        expect(searchMedia).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(screen.getByText("搜索失败，请重试。")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "重试" }));

      await waitFor(() => {
        expect(searchMedia).toHaveBeenCalledTimes(2);
        expect(screen.queryByText("搜索失败，请重试。")).not.toBeInTheDocument();
      });
    } finally {
      vi.useRealTimers();
      setSearchQuery("");
    }
  });
});
