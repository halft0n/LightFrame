import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { SearchResults } from "./SearchResults";
import { setLocale } from "@/i18n/index";
import { setSearchQuery } from "@/store/appStore";

const searchMedia = vi.fn();
const searchMediaCount = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    searchMedia: (...args: unknown[]) => searchMedia(...args),
    searchMediaCount: (...args: unknown[]) => searchMediaCount(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  setSearchQuery("");
  searchMedia.mockReset();
  searchMediaCount.mockReset();
});

describe("SearchResults", () => {
  it("shows no-results state when search query is empty", () => {
    render(<SearchResults />);
    expect(screen.getByText("未找到匹配结果")).toBeInTheDocument();
    expect(screen.getByText("试试其他关键词")).toBeInTheDocument();
  });

  it("shows loading state when searching", () => {
    setSearchQuery("sunset");
    searchMedia.mockReturnValue(new Promise(() => {}));
    searchMediaCount.mockReturnValue(new Promise(() => {}));

    render(<SearchResults />);
    expect(screen.getByText(/正在搜索|加载中|Loading/i)).toBeInTheDocument();
  });

  it("shows no-results state when search returns empty", async () => {
    setSearchQuery("nonexistent");
    searchMedia.mockResolvedValue([]);
    searchMediaCount.mockResolvedValue(0);

    render(<SearchResults />);
    await waitFor(() => {
      expect(screen.getByText("未找到匹配结果")).toBeInTheDocument();
    });
    expect(searchMedia).toHaveBeenCalledWith("nonexistent", 60, 0);
  });

  it("renders search results header", async () => {
    setSearchQuery("test");
    searchMedia.mockResolvedValue([]);
    searchMediaCount.mockResolvedValue(0);

    render(<SearchResults />);
    await waitFor(() => {
      expect(screen.getByText("搜索结果")).toBeInTheDocument();
    });
  });
});
