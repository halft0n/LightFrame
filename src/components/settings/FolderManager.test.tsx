import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FolderManager } from "./FolderManager";
import { setLocale } from "@/i18n/index";
import { setWatchedFolders } from "@/store/appStore";
import type { WatchedFolder } from "@/lib/tauri";

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
});
