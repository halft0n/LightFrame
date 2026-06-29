import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryDetailView } from "./MemoryDetailView";
import { setLocale } from "@/i18n/index";
import { closeMemoryDetail, openMemoryDetail } from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

const listMemories = vi.fn();
const getMemoryMedia = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listMemories: (...args: unknown[]) => listMemories(...args),
    getMemoryMedia: (...args: unknown[]) => getMemoryMedia(...args),
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
  openMemoryDetail(3);
  listMemories.mockReset();
  getMemoryMedia.mockReset();
});

describe("MemoryDetailView", () => {
  it("shows loading state initially", () => {
    listMemories.mockReturnValue(new Promise(() => {}));
    getMemoryMedia.mockReturnValue(new Promise(() => {}));

    render(<MemoryDetailView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("renders memory title and media count", async () => {
    listMemories.mockResolvedValue([
      {
        id: 3,
        title: "Summer Trip",
        subtitle: "Beach days",
        cover_media_id: 10,
        media_count: 1,
        date_from: "2024-07-01",
        date_to: "2024-07-15",
        created_at: "2024-08-01T00:00:00",
      },
    ]);
    getMemoryMedia.mockResolvedValue([sampleMedia]);

    render(<MemoryDetailView />);

    await waitFor(() => {
      expect(screen.getByText("Summer Trip")).toBeInTheDocument();
    });
    expect(screen.getByText("Beach days")).toBeInTheDocument();
    expect(screen.getByText("1 张照片")).toBeInTheDocument();
  });

  it("navigates back on back button click", async () => {
    const user = userEvent.setup();
    listMemories.mockResolvedValue([
      {
        id: 3,
        title: "Summer Trip",
        subtitle: null,
        cover_media_id: null,
        media_count: 0,
        date_from: "2024-07-01",
        date_to: "2024-07-15",
        created_at: "2024-08-01T00:00:00",
      },
    ]);
    getMemoryMedia.mockResolvedValue([]);

    render(<MemoryDetailView />);

    await waitFor(() => {
      expect(screen.getByText("← 返回")).toBeInTheDocument();
    });

    await user.click(screen.getByText("← 返回"));

    await waitFor(() => {
      expect(screen.queryByText("Summer Trip")).not.toBeInTheDocument();
    });
  });

  it("returns null when no memory is selected", () => {
    closeMemoryDetail();
    const { container } = render(<MemoryDetailView />);
    expect(container).toBeEmptyDOMElement();
  });

  it("loads memory media grid", async () => {
    listMemories.mockResolvedValue([
      {
        id: 3,
        title: "Weekend",
        subtitle: null,
        cover_media_id: 10,
        media_count: 1,
        date_from: "2024-01-01",
        date_to: "2024-01-07",
        created_at: "2024-02-01T00:00:00",
      },
    ]);
    getMemoryMedia.mockResolvedValue([sampleMedia]);

    render(<MemoryDetailView />);

    await waitFor(() => {
      expect(screen.getByText("Weekend")).toBeInTheDocument();
    });
    expect(getMemoryMedia).toHaveBeenCalledWith(3, 0, 60);
  });
});
