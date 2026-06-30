import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { FolderView } from "./FolderView";
import { setLocale } from "@/i18n/index";
import { navigate } from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

const getMediaByFolder = vi.fn();
const getMediaCountByFolder = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getMediaByFolder: (...args: unknown[]) => getMediaByFolder(...args),
    getMediaCountByFolder: (...args: unknown[]) =>
      getMediaCountByFolder(...args),
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
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

const sampleMedia: MediaItem = {
  id: 1,
  path: "/photos/sunset.jpg",
  filename: "sunset.jpg",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-01-01T00:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getMediaByFolder.mockReset();
  getMediaCountByFolder.mockReset();
});

describe("FolderView", () => {
  it("shows empty state when no folder is selected", () => {
    render(<FolderView />);
    expect(screen.getByText("暂无照片")).toBeInTheDocument();
  });

  it("shows loading state while fetching folder media", () => {
    navigate("folder", { folderId: 1, folderPath: "/photos/vacation" });
    getMediaByFolder.mockReturnValue(new Promise(() => {}));
    getMediaCountByFolder.mockReturnValue(new Promise(() => {}));

    render(<FolderView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("renders folder name and path after loading", async () => {
    navigate("folder", { folderId: 1, folderPath: "/photos/vacation" });
    getMediaByFolder.mockResolvedValue([sampleMedia]);
    getMediaCountByFolder.mockResolvedValue(1);

    render(<FolderView />);

    await waitFor(() => {
      expect(screen.getByText("文件夹")).toBeInTheDocument();
    });
    expect(screen.getByText("/photos/vacation")).toBeInTheDocument();
    expect(screen.getByText("共 1 项")).toBeInTheDocument();
  });

  it("renders folder media grid when items exist", async () => {
    navigate("folder", { folderId: 1, folderPath: "/photos/vacation" });
    getMediaByFolder.mockResolvedValue([sampleMedia]);
    getMediaCountByFolder.mockResolvedValue(1);

    render(<FolderView />);

    await waitFor(() => {
      expect(
        screen.getByRole("grid", { name: "照片网格" }),
      ).toBeInTheDocument();
    });
    expect(getMediaByFolder).toHaveBeenCalledWith(1, 0, 60);
  });

  it("handles empty folder", async () => {
    navigate("folder", { folderId: 2, folderPath: "/photos/empty" });
    getMediaByFolder.mockResolvedValue([]);
    getMediaCountByFolder.mockResolvedValue(0);

    render(<FolderView />);

    await waitFor(() => {
      expect(screen.getByText("暂无照片")).toBeInTheDocument();
    });
    expect(screen.getByText("/photos/empty")).toBeInTheDocument();
  });
});
