import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { FolderManager } from "./FolderManager";
import { setLocale } from "@/i18n/index";
import { setWatchedFolders } from "@/store/appStore";
import type { WatchedFolder } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const removeWatchedFolder = vi.fn();
const scanFolder = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    removeWatchedFolder: (...args: unknown[]) => removeWatchedFolder(...args),
    scanFolder: (...args: unknown[]) => scanFolder(...args),
  };
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@/hooks/useTheme", () => ({
  changeTheme: vi.fn(),
}));

const sampleFolder: WatchedFolder = {
  id: 1,
  path: "/home/user/photos",
  media_count: 42,
  scan_status: "idle",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  setWatchedFolders([]);
  removeWatchedFolder.mockReset();
  scanFolder.mockReset();
  vi.clearAllMocks();
  (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
    if (cmd === "get_log_config") {
      return Promise.resolve({ level: "debug", retention_days: 14, max_size_mb: 200 });
    }
    if (cmd === "get_log_directory") return Promise.resolve("/logs");
    if (cmd === "get_log_files") return Promise.resolve([]);
    return Promise.resolve(null);
  });
  delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("FolderManager", () => {
  it("renders empty state", () => {
    render(<FolderManager />);
    expect(screen.getByText("添加文件夹")).toBeInTheDocument();
  });

  it("renders folder list", () => {
    setWatchedFolders([sampleFolder]);

    render(<FolderManager />);
    expect(screen.getByText("photos")).toBeInTheDocument();
    expect(screen.getByText("/home/user/photos")).toBeInTheDocument();
    expect(screen.getByText("媒体数量: 42")).toBeInTheDocument();
  });

  it("add folder button is present", () => {
    render(<FolderManager />);
    expect(screen.getByRole("button", { name: "添加文件夹" })).toBeInTheDocument();
  });

  it("remove folder interaction calls backend", async () => {
    const user = userEvent.setup();
    setWatchedFolders([sampleFolder]);
    removeWatchedFolder.mockResolvedValue(undefined);

    render(<FolderManager />);
    await user.click(screen.getByRole("button", { name: "移除" }));

    expect(removeWatchedFolder).toHaveBeenCalledWith(1);
  });

  it("renders theme selector options", () => {
    render(<FolderManager />);
    expect(screen.getByText("浅色")).toBeInTheDocument();
    expect(screen.getByText("深色")).toBeInTheDocument();
    expect(screen.getByText("跟随系统")).toBeInTheDocument();
  });

  it("changes theme when option clicked", async () => {
    const user = userEvent.setup();
    const { changeTheme } = await import("@/hooks/useTheme");
    const changeThemeSpy = vi.mocked(changeTheme);

    render(<FolderManager />);
    await user.click(screen.getByRole("button", { name: "深色" }));

    expect(changeThemeSpy).toHaveBeenCalledWith("dark");
  });

  it("rescan folder button calls scanFolder", async () => {
    const user = userEvent.setup();
    setWatchedFolders([sampleFolder]);
    scanFolder.mockResolvedValue(undefined);

    render(<FolderManager />);
    await user.click(screen.getByRole("button", { name: "重新扫描" }));

    expect(scanFolder).toHaveBeenCalledWith(1);
  });

  it("shows scanning status while rescan is in progress", async () => {
    const user = userEvent.setup();
    setWatchedFolders([sampleFolder]);
    scanFolder.mockImplementation(
      () => new Promise((resolve) => setTimeout(resolve, 100)),
    );

    render(<FolderManager />);
    await user.click(screen.getByRole("button", { name: "重新扫描" }));

    expect(screen.getAllByText("扫描中…").length).toBeGreaterThan(0);
  });
});
