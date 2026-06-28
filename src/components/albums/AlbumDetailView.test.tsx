import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AlbumDetailView } from "./AlbumDetailView";
import { setLocale } from "@/i18n/index";
import { openAlbumDetail } from "@/store/appStore";
import type { Album, MediaItem } from "@/lib/tauri";

const listAlbums = vi.fn();
const getAlbumMedia = vi.fn();
const setAlbumCover = vi.fn();
const removeFromAlbum = vi.fn();
const getMediaList = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listAlbums: (...args: unknown[]) => listAlbums(...args),
    getAlbumMedia: (...args: unknown[]) => getAlbumMedia(...args),
    setAlbumCover: (...args: unknown[]) => setAlbumCover(...args),
    removeFromAlbum: (...args: unknown[]) => removeFromAlbum(...args),
    getMediaList: (...args: unknown[]) => getMediaList(...args),
  };
});

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
  id: 10,
  path: "/photos/a.jpg",
  filename: "a.jpg",
  media_type: "Photo",
  size_bytes: 1024,
  modified_at: "2024-01-01T00:00:00",
};

const sampleAlbum: Album = {
  id: 5,
  name: "Vacation",
  description: null,
  cover_media_id: null,
  media_count: 1,
  created_at: "2024-01-01",
  updated_at: "2024-01-01",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  openAlbumDetail(5);
  listAlbums.mockReset();
  getAlbumMedia.mockReset();
  setAlbumCover.mockReset();
  removeFromAlbum.mockReset();
  getMediaList.mockReset();
});

describe("AlbumDetailView", () => {
  it("renders album media grid", async () => {
    listAlbums.mockResolvedValue([sampleAlbum]);
    getAlbumMedia.mockResolvedValue([sampleMedia]);

    render(<AlbumDetailView />);
    await waitFor(() => {
      expect(screen.getByText("Vacation")).toBeInTheDocument();
    });
  });

  it("set cover photo action", async () => {
    const user = userEvent.setup();
    listAlbums.mockResolvedValue([sampleAlbum]);
    getAlbumMedia.mockResolvedValue([sampleMedia]);
    setAlbumCover.mockResolvedValue(undefined);

    render(<AlbumDetailView />);
    await waitFor(() => {
      expect(screen.getByTitle("设为封面")).toBeInTheDocument();
    });

    await user.click(screen.getByTitle("设为封面"));
    await waitFor(() => {
      expect(setAlbumCover).toHaveBeenCalledWith(5, 10);
    });
  });

  it("remove from album action", async () => {
    const user = userEvent.setup();
    listAlbums.mockResolvedValue([sampleAlbum]);
    getAlbumMedia.mockResolvedValue([sampleMedia]);
    removeFromAlbum.mockResolvedValue(undefined);

    render(<AlbumDetailView />);
    await waitFor(() => {
      expect(screen.getByTitle("移出相簿")).toBeInTheDocument();
    });

    await user.click(screen.getByTitle("移出相簿"));
    await waitFor(() => {
      expect(removeFromAlbum).toHaveBeenCalledWith(5, 10);
    });
  });

  it("shows empty album state", async () => {
    listAlbums.mockResolvedValue([{ ...sampleAlbum, media_count: 0 }]);
    getAlbumMedia.mockResolvedValue([]);

    render(<AlbumDetailView />);
    await waitFor(() => {
      expect(screen.getByText("暂无照片")).toBeInTheDocument();
    });
  });

  it("shows album name in header", async () => {
    listAlbums.mockResolvedValue([sampleAlbum]);
    getAlbumMedia.mockResolvedValue([]);

    render(<AlbumDetailView />);
    await waitFor(() => {
      expect(screen.getByText("Vacation")).toBeInTheDocument();
    });
  });
});
