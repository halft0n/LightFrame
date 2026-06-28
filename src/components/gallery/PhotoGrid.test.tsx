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
} from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor(private callback?: ResizeObserverCallback) {}
  trigger(width = 800) {
    this.callback?.(
      [{ contentRect: { width, height: 600 } } as ResizeObserverEntry],
      this as unknown as ResizeObserver,
    );
  }
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

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
  vi.clearAllMocks();
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
    setMedia([sampleMedia, { ...sampleMedia, id: 2, filename: "beach.jpg" }], 2);
    toggleMediaSelection(1);

    render(<PhotoGrid />);

    await waitFor(() => {
      expect(screen.getByText(/已选择 1 项/)).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "删除" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "加入相簿" })).toBeInTheDocument();
  });

  it("handles loadMore failure without crashing", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    setMedia(moreMedia, 120);

    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_page") return Promise.reject(new Error("network error"));
      if (cmd === "list_albums") return Promise.resolve([]);
      return Promise.resolve(null);
    });

    const { container } = render(<PhotoGrid />);
    const scrollContainer = container.querySelector(".overflow-y-auto");
    expect(scrollContainer).toBeTruthy();

    Object.defineProperty(scrollContainer!, "scrollHeight", { value: 2000, configurable: true });
    Object.defineProperty(scrollContainer!, "clientHeight", { value: 600, configurable: true });
    Object.defineProperty(scrollContainer!, "scrollTop", { value: 1300, writable: true, configurable: true });

    fireEvent.scroll(scrollContainer!);

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalledWith(
        "Failed to load more media:",
        expect.any(Error),
      );
    });
    expect(screen.queryByText("加载更多失败")).not.toBeInTheDocument();
    expect(getSnapshot().mediaItems).toHaveLength(60);

    consoleSpy.mockRestore();
  });
});
