import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { FavoritesView } from "./FavoritesView";
import { setLocale } from "@/i18n/index";

const getFavorites = vi.fn();
const getFavoritesCount = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getFavorites: (...args: unknown[]) => getFavorites(...args),
    getFavoritesCount: (...args: unknown[]) => getFavoritesCount(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getFavorites.mockReset();
  getFavoritesCount.mockReset();
});

describe("FavoritesView", () => {
  it("shows loading state initially", () => {
    getFavorites.mockReturnValue(new Promise(() => {}));
    getFavoritesCount.mockReturnValue(new Promise(() => {}));

    render(<FavoritesView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it('shows empty state when no favorites', async () => {
    getFavorites.mockResolvedValue([]);
    getFavoritesCount.mockResolvedValue(0);

    render(<FavoritesView />);
    await waitFor(() => {
      expect(screen.getByText("暂无收藏")).toBeInTheDocument();
    });
    expect(screen.getByText("点击❤️收藏喜欢的照片")).toBeInTheDocument();
  });

  it("renders favorites title after loading", async () => {
    getFavorites.mockResolvedValue([]);
    getFavoritesCount.mockResolvedValue(0);

    render(<FavoritesView />);
    await waitFor(() => {
      expect(screen.getByText("收藏")).toBeInTheDocument();
    });
  });
});
