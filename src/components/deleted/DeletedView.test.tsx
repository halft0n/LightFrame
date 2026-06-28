import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeletedView } from "./DeletedView";
import { setLocale } from "@/i18n/index";

const getDeletedMedia = vi.fn();
const restoreMedia = vi.fn();
const permanentlyDelete = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getDeletedMedia: (...args: unknown[]) => getDeletedMedia(...args),
    restoreMedia: (...args: unknown[]) => restoreMedia(...args),
    permanentlyDelete: (...args: unknown[]) => permanentlyDelete(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

const sampleMedia = {
  id: 99,
  path: "/photos/deleted.jpg",
  filename: "deleted.jpg",
  media_type: "Photo" as const,
  size_bytes: 512,
  modified_at: "2024-01-01T00:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getDeletedMedia.mockReset();
  restoreMedia.mockReset();
  permanentlyDelete.mockReset();
  vi.stubGlobal("confirm", vi.fn(() => true));
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

  it("renders deleted items with restore and delete actions", async () => {
    getDeletedMedia.mockResolvedValue([sampleMedia]);

    render(<DeletedView />);
    await waitFor(() => {
      expect(screen.getByText("共 1 项")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "恢复" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "永久删除" })).toBeInTheDocument();
  });

  it("restore button works", async () => {
    const user = userEvent.setup();
    getDeletedMedia.mockResolvedValue([sampleMedia]);
    restoreMedia.mockResolvedValue(undefined);
    getDeletedMedia.mockResolvedValueOnce([sampleMedia]).mockResolvedValueOnce([]);

    render(<DeletedView />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "恢复" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "恢复" }));
    await waitFor(() => {
      expect(restoreMedia).toHaveBeenCalledWith(99);
    });
  });

  it("permanent delete shows confirmation", async () => {
    const user = userEvent.setup();
    const confirmMock = vi.fn(() => false);
    vi.stubGlobal("confirm", confirmMock);
    getDeletedMedia.mockResolvedValue([sampleMedia]);

    render(<DeletedView />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "永久删除" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "永久删除" }));
    expect(confirmMock).toHaveBeenCalled();
    expect(permanentlyDelete).not.toHaveBeenCalled();
  });
});
