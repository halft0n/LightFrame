import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SmartAlbumView } from "./SmartAlbumView";
import { setLocale } from "@/i18n/index";
import { closeSmartAlbumDetail, openSmartAlbumDetail } from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

const listSmartAlbums = vi.fn();
const getSmartAlbumMedia = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listSmartAlbums: (...args: unknown[]) => listSmartAlbums(...args),
    getSmartAlbumMedia: (...args: unknown[]) => getSmartAlbumMedia(...args),
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

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  openSmartAlbumDetail(5);
  listSmartAlbums.mockReset();
  getSmartAlbumMedia.mockReset();
});

describe("SmartAlbumView", () => {
  it("shows loading state initially", () => {
    listSmartAlbums.mockReturnValue(new Promise(() => {}));
    getSmartAlbumMedia.mockReturnValue(new Promise(() => {}));

    render(<SmartAlbumView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("renders smart album media grid", async () => {
    listSmartAlbums.mockResolvedValue([
      {
        id: 5,
        name: "Nature",
        icon: "🌿",
        media_count: 1,
        rule: { media_type: "Photo" },
        created_at: "2024-01-01",
      },
    ]);
    getSmartAlbumMedia.mockResolvedValue([sampleMedia]);

    render(<SmartAlbumView />);

    await waitFor(() => {
      expect(screen.getByText("Nature")).toBeInTheDocument();
    });
    expect(screen.getByText("共 1 项")).toBeInTheDocument();
  });

  it("shows empty state when no media", async () => {
    listSmartAlbums.mockResolvedValue([
      {
        id: 5,
        name: "Empty",
        icon: "📂",
        media_count: 0,
        rule: { media_type: "Photo" },
        created_at: "2024-01-01",
      },
    ]);
    getSmartAlbumMedia.mockResolvedValue([]);

    render(<SmartAlbumView />);

    await waitFor(() => {
      expect(screen.getByText("暂无照片")).toBeInTheDocument();
    });
  });

  it("shows error state when loading fails", async () => {
    listSmartAlbums.mockRejectedValue(new Error("fail"));
    getSmartAlbumMedia.mockRejectedValue(new Error("fail"));

    render(<SmartAlbumView />);

    await waitFor(() => {
      expect(screen.getByText("操作失败，请重试")).toBeInTheDocument();
    });
  });

  it("navigates back on back button click", async () => {
    const user = userEvent.setup();
    listSmartAlbums.mockResolvedValue([
      {
        id: 5,
        name: "Nature",
        icon: "🌿",
        media_count: 0,
        rule: { media_type: "Photo" },
        created_at: "2024-01-01",
      },
    ]);
    getSmartAlbumMedia.mockResolvedValue([]);

    render(<SmartAlbumView />);

    await waitFor(() => {
      expect(screen.getByText("← 返回")).toBeInTheDocument();
    });

    await user.click(screen.getByText("← 返回"));

    await waitFor(() => {
      expect(screen.queryByText("Nature")).not.toBeInTheDocument();
    });
  });

  it("returns null when no smart album is selected", () => {
    closeSmartAlbumDetail();
    const { container } = render(<SmartAlbumView />);
    expect(container).toBeEmptyDOMElement();
  });
});
