import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { DeletedView } from "./DeletedView";
import { setLocale } from "@/i18n/index";

const getDeletedMedia = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getDeletedMedia: (...args: unknown[]) => getDeletedMedia(...args),
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
  getDeletedMedia.mockReset();
});

describe("DeletedView", () => {
  it("shows loading state initially", () => {
    getDeletedMedia.mockReturnValue(new Promise(() => {}));

    render(<DeletedView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it('shows empty state when no deleted items', async () => {
    getDeletedMedia.mockResolvedValue([]);

    render(<DeletedView />);
    await waitFor(() => {
      expect(screen.getByText("最近删除为空")).toBeInTheDocument();
    });
    expect(screen.getByText("删除的照片会在此保留30天")).toBeInTheDocument();
  });

  it("renders deleted title and notice after loading", async () => {
    getDeletedMedia.mockResolvedValue([]);

    render(<DeletedView />);
    await waitFor(() => {
      expect(screen.getByText("最近删除")).toBeInTheDocument();
    });
    expect(screen.getByText("照片将在删除30天后永久清除")).toBeInTheDocument();
  });
});
