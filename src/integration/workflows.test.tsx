import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "@/App";
import { setLocale } from "@/i18n/index";
import {
  clearMediaSelection,
  getSnapshot,
  setMediaSelection,
  setSearchQuery,
} from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

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
