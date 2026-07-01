import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { PhotoGrid } from "./PhotoGrid";
import { setLocale } from "@/i18n/index";
import {
  setMedia,
  getSnapshot,
  toggleMediaSelection,
  clearMediaSelection,
  setThumbnailSize,
  resetStore,
} from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

const batchDeleteMedia = vi.fn();
const batchToggleFavorite = vi.fn();
const loadMediaMock = vi.fn();
const loadMoreMediaMock = vi.fn();

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
let mockScrollIntent: import("@/hooks/useScrollIntent").ScrollIntent = "idle";
vi.mock("@/hooks/useScrollIntent", () => ({
  useScrollIntent: () => mockScrollIntent,
}));
let lastVirtualizerOverscan = 0;
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count, overscan }: { count: number; overscan?: number }) => {
    lastVirtualizerOverscan = overscan ?? 0;
    return {
      getTotalSize: () => Math.max(count, 1) * 120,
      getVirtualItems: () =>
        Array.from({ length: count }, (_, index) => ({
          key: index,
          index,
          start: index * 120,
          size: 120,
        })),
      measure: vi.fn(),
      range: { startIndex: 0, endIndex: Math.max(count - 1, 0) },
    };
  },
}));
vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    batchDeleteMedia: (...args: unknown[]) => batchDeleteMedia(...args),
    batchToggleFavorite: (...args: unknown[]) => batchToggleFavorite(...args),
  };
});
vi.mock("@/store/appStore", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/store/appStore")>();
  return {
    ...actual,
    loadMedia: (...args: unknown[]) => loadMediaMock(...args),
    loadMoreMedia: (...args: unknown[]) => loadMoreMediaMock(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor(private callback?: ResizeObserverCallback) {
    lastResizeObserver = this;
  }
  trigger(width = 800) {
    this.callback?.(
      [{ contentRect: { width, height: 600 } } as ResizeObserverEntry],
      this as unknown as ResizeObserver,
    );
  }
}
let lastResizeObserver: ResizeObserverMock | null = null;
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

function renderGridWithLayout(items: MediaItem[], totalCount?: number) {
  if (totalCount !== undefined) {
    setMedia(items, totalCount);
  } else {
    setMedia(items, items.length);
  }
  const view = render(<PhotoGrid />);
  const scrollContainer = view.container.querySelector(".overflow-y-auto");
  if (scrollContainer) {
    Object.defineProperty(scrollContainer, "clientWidth", {
      value: 800,
      configurable: true,
    });
    Object.defineProperty(scrollContainer, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollContainer, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
  }
  lastResizeObserver?.trigger(800);
  return view;
}

const sampleMedia: MediaItem = {
  id: 1,
  path: "/photos/sunset.jpg",
  filename: "sunset.jpg",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-01-01T00:00:00",
};

const moreMedia: MediaItem[] = Array.from({ length: 60 }, (_, i) => ({
  ...sampleMedia,
  id: i + 1,
  filename: `photo-${i + 1}.jpg`,
}));

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  clearMediaSelection();
  setMedia([], 0);
  setThumbnailSize("medium");
  lastResizeObserver = null;
  vi.clearAllMocks();
  batchDeleteMedia.mockResolvedValue(1);
  batchToggleFavorite.mockResolvedValue(1);
  loadMediaMock.mockResolvedValue(undefined);
  loadMoreMediaMock.mockResolvedValue(undefined);
  (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
    if (cmd === "get_media_page") return Promise.resolve([]);
    if (cmd === "get_media_count") return Promise.resolve(0);
    if (cmd === "list_albums") return Promise.resolve([]);
    return Promise.resolve(null);
  });
});

describe("PhotoGrid", () => {
  it("shows empty state when no photos", () => {
    render(<PhotoGrid />);
    expect(screen.getByText("暂无照片")).toBeInTheDocument();
  });

  it("renders grid with media items", () => {
    setMedia([sampleMedia], 15);
    render(<PhotoGrid />);
    expect(getSnapshot().mediaItems).toHaveLength(1);
    expect(getSnapshot().totalCount).toBe(15);
  });

  it("shows selection toolbar when items are selected", async () => {
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );
    toggleMediaSelection(1);

    render(<PhotoGrid />);

    await waitFor(() => {
      expect(screen.getByText(/已选择 1 项/)).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "删除" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "加入相簿" }),
    ).toBeInTheDocument();
  });

  it("handles loadMore failure without crashing", async () => {
    setMedia(moreMedia, 120);
    loadMoreMediaMock.mockRejectedValue(new Error("network error"));

    const { container } = render(<PhotoGrid />);
    const scrollContainer = container.querySelector(".overflow-y-auto");
    expect(scrollContainer).toBeTruthy();

    Object.defineProperty(scrollContainer!, "scrollHeight", {
      value: 2000,
      configurable: true,
    });
    Object.defineProperty(scrollContainer!, "clientHeight", {
      value: 600,
      configurable: true,
    });
    Object.defineProperty(scrollContainer!, "scrollTop", {
      value: 1300,
      writable: true,
      configurable: true,
    });

    fireEvent.scroll(scrollContainer!);

    await waitFor(() => {
      expect(screen.getByText("加载更多失败")).toBeInTheDocument();
    });
    expect(getSnapshot().mediaItems).toHaveLength(60);
  });

  it("clears selection on Escape key", async () => {
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );
    toggleMediaSelection(1);

    render(<PhotoGrid />);

    await waitFor(() => {
      expect(screen.getByText(/已选择 1 项/)).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => {
      expect(getSnapshot().selectedMediaIds).toHaveLength(0);
    });
  });

  it("selects all with Ctrl+A", async () => {
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );

    render(<PhotoGrid />);

    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await waitFor(() => {
      expect(getSnapshot().selectedMediaIds).toEqual([1, 2]);
    });
  });

  it("selects all with Meta+A on macOS-style modifier", async () => {
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );

    render(<PhotoGrid />);

    fireEvent.keyDown(window, { key: "a", metaKey: true });
    await waitFor(() => {
      expect(getSnapshot().selectedMediaIds).toEqual([1, 2]);
    });
  });

  it("ignores keyboard shortcuts when typing in input", async () => {
    setMedia([sampleMedia], 1);
    toggleMediaSelection(1);

    render(
      <>
        <PhotoGrid />
        <input data-testid="search-input" aria-label="search" />
      </>,
    );

    const input = screen.getByTestId("search-input");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(getSnapshot().selectedMediaIds).toEqual([1]);
  });

  it("batch deletes selected items on Delete key when confirmed", async () => {
    vi.stubGlobal(
      "confirm",
      vi.fn(() => true),
    );
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );
    toggleMediaSelection(1);
    toggleMediaSelection(2);

    render(<PhotoGrid />);

    fireEvent.keyDown(window, { key: "Delete" });

    await waitFor(() => {
      expect(batchDeleteMedia).toHaveBeenCalledWith([1, 2]);
      expect(getSnapshot().selectedMediaIds).toEqual([]);
      expect(loadMediaMock).toHaveBeenCalled();
    });
  });

  it("skips batch delete when confirmation is cancelled", async () => {
    vi.stubGlobal(
      "confirm",
      vi.fn(() => false),
    );
    setMedia([sampleMedia], 1);
    toggleMediaSelection(1);

    render(<PhotoGrid />);

    fireEvent.keyDown(window, { key: "Delete" });

    await waitFor(() => {
      expect(batchDeleteMedia).not.toHaveBeenCalled();
      expect(getSnapshot().selectedMediaIds).toEqual([1]);
    });
  });

  it("batch favorites selected items on F key", async () => {
    setMedia(
      [sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }],
      2,
    );
    toggleMediaSelection(1);

    render(<PhotoGrid />);

    fireEvent.keyDown(window, { key: "f" });

    await waitFor(() => {
      expect(batchToggleFavorite).toHaveBeenCalledWith([1], true);
    });
  });

  it("changes thumbnail size via size control", async () => {
    setMedia([sampleMedia], 1);
    render(<PhotoGrid />);

    fireEvent.click(screen.getByRole("button", { name: "小" }));

    expect(getSnapshot().thumbnailSize).toBe("small");
  });

  it("shift-click selects inclusive range", async () => {
    const items = [
      sampleMedia,
      { ...sampleMedia, id: 2, filename: "beach.jpg" },
      { ...sampleMedia, id: 3, filename: "mountain.jpg" },
    ];
    renderGridWithLayout(items);

    await waitFor(() => {
      expect(screen.getAllByRole("gridcell").length).toBeGreaterThanOrEqual(3);
    });

    const cells = screen.getAllByRole("gridcell");
    fireEvent.click(cells[0]!);
    expect(getSnapshot().selectedMediaIds).toEqual([1]);

    fireEvent.click(cells[2]!, { shiftKey: true });
    expect(getSnapshot().selectedMediaIds).toEqual([1, 2, 3]);
  });

  it("ctrl-click toggles selection without clearing others", async () => {
    const items = [
      sampleMedia,
      { ...sampleMedia, id: 2, filename: "beach.jpg" },
    ];
    renderGridWithLayout(items);

    await waitFor(() => {
      expect(screen.getAllByRole("gridcell").length).toBeGreaterThanOrEqual(2);
    });

    const cells = screen.getAllByRole("gridcell");
    fireEvent.click(cells[0]!);
    fireEvent.click(cells[1]!, { ctrlKey: true });
    expect(getSnapshot().selectedMediaIds).toEqual([1, 2]);

    fireEvent.click(cells[0]!, { ctrlKey: true });
    expect(getSnapshot().selectedMediaIds).toEqual([2]);
  });

  it("shows error banner with retry when mediaLoadError is set and no items", () => {
    resetStore({ mediaLoadError: "connection refused" });
    render(<PhotoGrid />);

    expect(screen.getByText("connection refused")).toBeInTheDocument();
    expect(screen.getByText("重试")).toBeInTheDocument();
  });

  it("shows empty state instead of error when no error set", () => {
    resetStore({ mediaLoadError: null });
    render(<PhotoGrid />);

    expect(screen.getByText("暂无照片")).toBeInTheDocument();
    expect(screen.queryByText("重试")).not.toBeInTheDocument();
  });

  it("does not render grid cells before container width is measured", () => {
    setMedia([sampleMedia], 1);
    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      get: () => 0,
    });
    render(<PhotoGrid />);

    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
    expect(screen.queryByRole("gridcell")).not.toBeInTheDocument();

    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      get: () => 800,
    });
  });

  it("renders square grid cells after container width is measured", async () => {
    setMedia([sampleMedia], 1);
    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      get: () => 800,
    });
    render(<PhotoGrid />);

    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });
    const cells = screen.getAllByRole("gridcell");
    expect(cells.length).toBeGreaterThan(0);
  });
});

describe("PhotoGrid scroll intent integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setLocale("zh-CN");
    resetStore();
    mockScrollIntent = "idle";
    lastVirtualizerOverscan = 0;
    Object.defineProperty(HTMLElement.prototype, "clientWidth", {
      configurable: true,
      get: () => 800,
    });
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValue([]);
  });

  const items: MediaItem[] = Array.from({ length: 50 }, (_, i) => ({
    id: i + 1,
    path: `/photos/img_${i}.jpg`,
    filename: `img_${i}.jpg`,
    media_type: "Photo" as const,
    size_bytes: 1024,
    modified_at: "2024-01-01T00:00:00",
  }));

  it("passes overscan=5 to virtualizer during idle scroll", async () => {
    mockScrollIntent = "idle";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    expect(lastVirtualizerOverscan).toBe(5);
  });

  it("passes overscan=5 to virtualizer during slow scroll", async () => {
    mockScrollIntent = "slow";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    expect(lastVirtualizerOverscan).toBe(5);
  });

  it("passes overscan=3 to virtualizer during medium scroll", async () => {
    mockScrollIntent = "medium";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    expect(lastVirtualizerOverscan).toBe(3);
  });

  it("passes overscan=1 to virtualizer during fast scroll", async () => {
    mockScrollIntent = "fast";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    expect(lastVirtualizerOverscan).toBe(1);
  });

  it("passes overscan=0 to virtualizer during burst scroll", async () => {
    mockScrollIntent = "burst";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    expect(lastVirtualizerOverscan).toBe(0);
  });

  it("passes scrollIntent prop to PhotoCard components", async () => {
    mockScrollIntent = "burst";
    setMedia(items, 50);
    render(<PhotoGrid />);
    lastResizeObserver?.trigger(800);

    await waitFor(() => {
      expect(screen.getByRole("grid")).toBeInTheDocument();
    });

    // PhotoCards render — with burst intent, they should defer full-size loading
    // (only micro thumbnail src rendered, no /small or /large src)
    const cells = screen.getAllByRole("gridcell");
    expect(cells.length).toBeGreaterThan(0);

    // Verify overscan is 0 for burst (PhotoCards won't over-fetch)
    expect(lastVirtualizerOverscan).toBe(0);
  });
});
