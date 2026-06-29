import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SearchResultsView } from "./SearchResultsView";
import { setLocale } from "@/i18n/index";
import { addSearchHistory, setSearchQuery, setSearchMode } from "@/store/appStore";

const searchMedia = vi.fn();
const searchMediaCount = vi.fn();
const semanticSearch = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    searchMedia: (...args: unknown[]) => searchMedia(...args),
    searchMediaCount: (...args: unknown[]) => searchMediaCount(...args),
    semanticSearch: (...args: unknown[]) => semanticSearch(...args),
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

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  setSearchQuery("");
  setSearchMode("text");
  searchMedia.mockReset();
  searchMediaCount.mockReset();
  semanticSearch.mockReset();
});

describe("SearchResultsView", () => {
  it("text search mode shows results", async () => {
    setSearchQuery("sunset");
    searchMedia.mockResolvedValue([
      {
        id: 1,
        path: "/photos/sunset.jpg",
        filename: "sunset.jpg",
        media_type: "Photo",
        size_bytes: 1024,
        modified_at: "2024-01-01T00:00:00",
      },
    ]);
    searchMediaCount.mockResolvedValue(1);

    render(<SearchResultsView />);
    await waitFor(() => {
      expect(screen.getByText("搜索结果")).toBeInTheDocument();
      expect(screen.getByText(/sunset/)).toBeInTheDocument();
    });
    expect(searchMedia).toHaveBeenCalledWith("sunset", 60, 0);
  });

  it("shows empty results state", async () => {
    setSearchQuery("nonexistent");
    searchMedia.mockResolvedValue([]);
    searchMediaCount.mockResolvedValue(0);

    render(<SearchResultsView />);
    await waitFor(() => {
      expect(screen.getByText("未找到匹配结果")).toBeInTheDocument();
    });
  });

  it("search mode toggle switches to semantic search", async () => {
    const user = userEvent.setup();
    setSearchQuery("beach");
    searchMedia.mockResolvedValue([]);
    searchMediaCount.mockResolvedValue(0);
    semanticSearch.mockResolvedValue({
      used_semantic: true,
      results: [
        {
          media_id: 5,
          file_path: "/photos/beach.jpg",
          file_name: "beach.jpg",
          relevance: 0.88,
        },
      ],
    });

    render(<SearchResultsView />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "语义搜索" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "语义搜索" }));

    await waitFor(() => {
      expect(semanticSearch).toHaveBeenCalledWith("beach", 60);
      expect(screen.getByText(/相似度: 88%/)).toBeInTheDocument();
    });
  });

  it("search history is stored when query is added via store", async () => {
    addSearchHistory("mountains");
    setSearchQuery("mountains");
    searchMedia.mockResolvedValue([]);
    searchMediaCount.mockResolvedValue(0);

    render(<SearchResultsView />);
    await waitFor(() => {
      expect(screen.getByText(/mountains/)).toBeInTheDocument();
    });
  });
});
