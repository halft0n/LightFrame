import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SelectionToolbar } from "./SelectionToolbar";
import { setLocale } from "@/i18n/index";
import {
  clearMediaSelection,
  setMediaSelection,
  navigate,
  openAlbumDetail,
} from "@/store/appStore";

const batchDeleteMedia = vi.fn();
const batchToggleFavorite = vi.fn();
const batchAddToAlbum = vi.fn();
const batchExport = vi.fn();
const removeFromAlbum = vi.fn();
const listAlbums = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    batchDeleteMedia: (...args: unknown[]) => batchDeleteMedia(...args),
    batchToggleFavorite: (...args: unknown[]) => batchToggleFavorite(...args),
    batchAddToAlbum: (...args: unknown[]) => batchAddToAlbum(...args),
    batchExport: (...args: unknown[]) => batchExport(...args),
    removeFromAlbum: (...args: unknown[]) => removeFromAlbum(...args),
    listAlbums: (...args: unknown[]) => listAlbums(...args),
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@/store/appStore", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/store/appStore")>();
  return {
    ...actual,
    loadMedia: vi.fn().mockResolvedValue(undefined),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  clearMediaSelection();
  navigate("all");
  batchDeleteMedia.mockReset();
  batchToggleFavorite.mockReset();
  batchAddToAlbum.mockReset();
  batchExport.mockReset();
  removeFromAlbum.mockReset();
  listAlbums.mockReset();
  listAlbums.mockResolvedValue([]);
});

describe("SelectionToolbar", () => {
  it("renders nothing when no items are selected", () => {
    const { container } = render(<SelectionToolbar />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows correct selection count", () => {
    setMediaSelection([1, 2, 3]);
    render(<SelectionToolbar />);
    expect(screen.getByText("已选择 3 项")).toBeInTheDocument();
  });

  it("renders action buttons", () => {
    setMediaSelection([1]);
    render(<SelectionToolbar />);

    expect(screen.getByText("删除")).toBeInTheDocument();
    expect(screen.getByText("加入相簿")).toBeInTheDocument();
    expect(screen.getByText("收藏")).toBeInTheDocument();
    expect(screen.getByText("导出")).toBeInTheDocument();
    expect(screen.getByText("取消")).toBeInTheDocument();
  });

  it("handles batch favorite operation", async () => {
    const user = userEvent.setup();
    setMediaSelection([1, 2]);
    batchToggleFavorite.mockResolvedValue(undefined);

    render(<SelectionToolbar />);
    await user.click(screen.getByText("收藏"));

    await waitFor(() => {
      expect(batchToggleFavorite).toHaveBeenCalledWith([1, 2], true);
    });
  });

  it("shows delete confirmation and performs batch delete", async () => {
    const user = userEvent.setup();
    setMediaSelection([5, 6]);
    batchDeleteMedia.mockResolvedValue(undefined);

    render(<SelectionToolbar />);
    await user.click(screen.getByText("删除"));

    expect(screen.getByText("确定要删除选中的 2 项吗？")).toBeInTheDocument();

    await user.click(screen.getByText("确定"));

    await waitFor(() => {
      expect(batchDeleteMedia).toHaveBeenCalledWith([5, 6]);
    });
  });

  it("shows album picker and adds to album", async () => {
    const user = userEvent.setup();
    setMediaSelection([1]);
    listAlbums.mockResolvedValue([{ id: 10, name: "Vacation", media_count: 5, created_at: "" }]);
    batchAddToAlbum.mockResolvedValue(undefined);

    render(<SelectionToolbar />);

    await waitFor(() => {
      expect(listAlbums).toHaveBeenCalled();
    });

    await user.click(screen.getByText("加入相簿"));
    await user.click(screen.getByText("Vacation"));

    await waitFor(() => {
      expect(batchAddToAlbum).toHaveBeenCalledWith(10, [1]);
    });
  });

  it("shows remove from album in album detail context", async () => {
    const user = userEvent.setup();
    openAlbumDetail(99);
    setMediaSelection([7]);
    removeFromAlbum.mockResolvedValue(undefined);

    render(<SelectionToolbar onAlbumChanged={vi.fn()} />);

    expect(screen.getByText("移出相簿")).toBeInTheDocument();
    expect(screen.queryByText("加入相簿")).not.toBeInTheDocument();

    await user.click(screen.getByText("移出相簿"));

    await waitFor(() => {
      expect(removeFromAlbum).toHaveBeenCalledWith(99, 7);
    });
  });

  it("clears selection when cancel is clicked", async () => {
    const user = userEvent.setup();
    setMediaSelection([1, 2]);

    render(<SelectionToolbar />);
    await user.click(screen.getByText("取消"));

    expect(screen.queryByText("已选择 2 项")).not.toBeInTheDocument();
  });
});
