import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Sidebar } from "./Sidebar";
import { setLocale } from "@/i18n/index";
import {
  navigate,
  setWatchedFolders,
  getSnapshot,
  clearMediaSelection,
} from "@/store/appStore";
import { DRAG_MEDIA_MIME } from "@/lib/dragMedia";

const listAlbums = vi.fn();
const addToAlbum = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listAlbums: () => listAlbums(),
    addToAlbum: (...args: Parameters<typeof actual.addToAlbum>) => addToAlbum(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  clearMediaSelection();
  navigate("all");
  setWatchedFolders([]);
  listAlbums.mockReset();
  addToAlbum.mockReset();
  listAlbums.mockResolvedValue([]);
});

describe("Sidebar", () => {
  it("renders the library section header", () => {
    render(<Sidebar />);
    expect(screen.getAllByText("基础图库").length).toBeGreaterThanOrEqual(1);
  });

  it("renders library section in Chinese", () => {
    render(<Sidebar />);
    expect(screen.getByText("所有照片")).toBeInTheDocument();
    expect(screen.getByText("视频")).toBeInTheDocument();
    expect(screen.getByText("时间线")).toBeInTheDocument();
    expect(screen.getByText("收藏")).toBeInTheDocument();
    expect(screen.getByText("地点")).toBeInTheDocument();
    expect(screen.getByText("人物")).toBeInTheDocument();
  });

  it("renders albums section in Chinese", () => {
    render(<Sidebar />);
    expect(screen.getByText("回忆")).toBeInTheDocument();
    expect(screen.getByText("重复照片")).toBeInTheDocument();
    expect(screen.getByText("截图")).toBeInTheDocument();
  });

  it("renders settings button", () => {
    render(<Sidebar />);
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("renders in English when locale is en", () => {
    setLocale("en");
    render(<Sidebar />);
    expect(screen.getByText("All Photos")).toBeInTheDocument();
    expect(screen.getByText("Videos")).toBeInTheDocument();
    expect(screen.getByText("Timeline")).toBeInTheDocument();
    expect(screen.getByText("Memories")).toBeInTheDocument();
    expect(screen.getByText("Duplicates")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders all navigation items as buttons", () => {
    render(<Sidebar />);
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBeGreaterThanOrEqual(7);
  });

  it("has navigation role and aria-label", () => {
    render(<Sidebar />);
    const nav = screen.getByRole("navigation", { name: "主导航" });
    expect(nav).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "主导航" })).toBeInTheDocument();
  });

  it("highlights active view with aria-current", () => {
    navigate("favorites");
    render(<Sidebar />);

    const favoritesButton = screen.getByRole("button", { name: "收藏" });
    expect(favoritesButton).toHaveAttribute("aria-current", "page");
  });

  it("shows folders section when watched folders exist", () => {
    setWatchedFolders([
      {
        id: 1,
        path: "/photos/vacation",
        media_count: 10,
        scan_status: "idle",
      },
    ]);
    render(<Sidebar />);

    expect(screen.getByText("文件夹")).toBeInTheDocument();
    expect(screen.getByText("vacation")).toBeInTheDocument();
  });

  it("highlights active folder", () => {
    setWatchedFolders([
      {
        id: 42,
        path: "/photos/trip",
        media_count: 3,
        scan_status: "idle",
      },
    ]);
    navigate("folder", { folderId: 42, folderPath: "/photos/trip" });
    render(<Sidebar />);

    const folderButton = screen.getByRole("button", { name: "trip" });
    expect(folderButton).toHaveAttribute("aria-current", "page");
  });

  it("shows albums section with user albums", async () => {
    listAlbums.mockResolvedValue([
      { id: 1, name: "Summer", media_count: 8, created_at: "2024-01-01" },
    ]);
    render(<Sidebar />);

    expect(screen.getAllByText("相簿").length).toBeGreaterThanOrEqual(1);
    expect(await screen.findByText("我的相簿")).toBeInTheDocument();
    expect(await screen.findByText("Summer")).toBeInTheDocument();
  });

  it("handles drag-over for albums", async () => {
    listAlbums.mockResolvedValue([
      { id: 5, name: "Drop Target", media_count: 0, created_at: "2024-01-01" },
    ]);
    render(<Sidebar />);

    const albumButton = await screen.findByRole("button", { name: /Drop Target/ });
    const dataTransfer = {
      types: [DRAG_MEDIA_MIME],
      dropEffect: "",
      preventDefault: vi.fn(),
      getData: () => JSON.stringify([1, 2]),
    };

    fireEvent.dragOver(albumButton, { dataTransfer });
    expect(albumButton.className).toContain("ring-blue-500");

    fireEvent.dragLeave(albumButton);
    expect(albumButton.className).not.toContain("ring-blue-500");
  });

  it("adds media to album on drop", async () => {
    addToAlbum.mockResolvedValue(undefined);
    listAlbums.mockResolvedValue([
      { id: 5, name: "Drop Target", media_count: 0, created_at: "2024-01-01" },
    ]);
    render(<Sidebar />);

    const albumButton = await screen.findByRole("button", { name: /Drop Target/ });
    const dataTransfer = {
      types: [DRAG_MEDIA_MIME],
      dropEffect: "",
      preventDefault: vi.fn(),
      getData: () => JSON.stringify([1, 2]),
    };

    fireEvent.drop(albumButton, { dataTransfer });

    await vi.waitFor(() => {
      expect(addToAlbum).toHaveBeenCalledWith(5, [1, 2]);
    });
  });

  it("navigates when a library item is clicked", async () => {
    const user = userEvent.setup();
    render(<Sidebar />);

    await user.click(screen.getByRole("button", { name: "时间线" }));
    expect(getSnapshot().currentView).toBe("timeline");
  });

  it("renders mobile sidebar when open", () => {
    const onClose = vi.fn();
    render(<Sidebar isMobile mobileOpen onMobileClose={onClose} />);

    expect(screen.getByText("影迹")).toBeInTheDocument();
    expect(screen.getByLabelText("关闭侧边栏")).toBeInTheDocument();
  });
});
