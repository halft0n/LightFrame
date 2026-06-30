import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { setLocale } from "@/i18n/index";
import { getSnapshot, clearMediaSelection } from "@/store/appStore";

const listWatchedFolders = vi.fn();
const onScanProgress = vi.fn();
const onFolderChanged = vi.fn();
const scanFolder = vi.fn();
const getFavorites = vi.fn();
const getFavoritesCount = vi.fn();
const getMediaByType = vi.fn();
const getMediaCountByType = vi.fn();
const listMemories = vi.fn();
const getOnThisDay = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listWatchedFolders: () => listWatchedFolders(),
    onScanProgress: (cb: Parameters<typeof actual.onScanProgress>[0]) =>
      onScanProgress(cb),
    onFolderChanged: (cb: Parameters<typeof actual.onFolderChanged>[0]) =>
      onFolderChanged(cb),
    scanFolder: (...args: Parameters<typeof actual.scanFolder>) =>
      scanFolder(...args),
    getFavorites: (...args: unknown[]) => getFavorites(...args),
    getFavoritesCount: (...args: unknown[]) => getFavoritesCount(...args),
    getMediaByType: (...args: unknown[]) => getMediaByType(...args),
    getMediaCountByType: (...args: unknown[]) => getMediaCountByType(...args),
    listMemories: (...args: unknown[]) => listMemories(...args),
    getOnThisDay: (...args: unknown[]) => getOnThisDay(...args),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "get_media_page") return Promise.resolve([]);
    if (cmd === "get_media_count") return Promise.resolve(0);
    if (cmd === "list_albums") return Promise.resolve([]);
    return Promise.resolve(null);
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

function setupMatchMedia(matches = false) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: query.includes("767px") ? matches : false,
    media: query,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

const watchedFolder = {
  id: 1,
  path: "/photos",
  media_count: 5,
  scan_status: "idle" as const,
};

function setupDefaultMocks() {
  listWatchedFolders.mockResolvedValue([watchedFolder]);
  onScanProgress.mockResolvedValue(() => {});
  onFolderChanged.mockResolvedValue(() => {});
  getFavorites.mockResolvedValue([]);
  getFavoritesCount.mockResolvedValue(0);
  getMediaByType.mockResolvedValue([]);
  getMediaCountByType.mockResolvedValue(0);
  listMemories.mockResolvedValue([]);
  getOnThisDay.mockResolvedValue([]);
}

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  clearMediaSelection();
  setupMatchMedia(false);
  vi.clearAllMocks();
  setupDefaultMocks();
});

describe("App", () => {
  it("renders main content area", async () => {
    render(<App />);

    await waitFor(() => {
      expect(document.querySelector("main")).toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText("搜索照片…")).toBeInTheDocument();
    expect(document.querySelector(".main-content-enter")).toBeInTheDocument();
  });

  it("shows hamburger button on mobile viewport", async () => {
    setupMatchMedia(true);
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("打开菜单")).toBeInTheDocument();
    });
  });

  it("opens and closes mobile sidebar overlay", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByLabelText("打开菜单")).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText("打开菜单"));
    expect(screen.getByLabelText("打开菜单")).toHaveAttribute(
      "aria-expanded",
      "true",
    );

    const backdrop = screen.getByLabelText("关闭侧边栏", {
      selector: ".sidebar-overlay-backdrop",
    });
    await user.click(backdrop);

    await waitFor(() => {
      expect(screen.getByLabelText("打开菜单")).toHaveAttribute(
        "aria-expanded",
        "false",
      );
    });
  });

  it("navigates between photos and favorites views", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(getSnapshot().currentView).toBe("all");
    });

    const nav = screen.getByRole("navigation", { name: "主导航" });
    await user.click(within(nav).getByText("收藏"));

    await waitFor(() => {
      expect(getSnapshot().currentView).toBe("favorites");
    });
    expect(await screen.findByText("暂无收藏")).toBeInTheDocument();
  });

  it("navigates to videos view", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("所有照片")).toBeInTheDocument();
    });

    const nav = screen.getByRole("navigation", { name: "主导航" });
    await user.click(within(nav).getByText("视频"));

    await waitFor(() => {
      expect(getSnapshot().currentView).toBe("videos");
    });
    expect(await screen.findByText("暂无视频")).toBeInTheDocument();
  });

  it("navigates to memories view", async () => {
    const user = userEvent.setup();
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("回忆")).toBeInTheDocument();
    });

    const nav = screen.getByRole("navigation", { name: "主导航" });
    await user.click(within(nav).getByText("回忆"));

    await waitFor(() => {
      expect(getSnapshot().currentView).toBe("memories");
    });
    expect(await screen.findByText("暂无回忆")).toBeInTheDocument();
  });

  it("shows welcome state when no folders are watched", async () => {
    listWatchedFolders.mockResolvedValue([]);
    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("欢迎使用 影迹")).toBeInTheDocument();
    });
  });
});
