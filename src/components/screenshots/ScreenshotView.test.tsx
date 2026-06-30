import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScreenshotView } from "./ScreenshotView";
import { setLocale } from "@/i18n/index";
import type { MediaItem } from "@/lib/tauri";

const getScreenshots = vi.fn();
const getScreenshotCount = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getScreenshots: (...args: unknown[]) => getScreenshots(...args),
    getScreenshotCount: (...args: unknown[]) => getScreenshotCount(...args),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn(
    (filePath: string, protocol: string = "asset") =>
      `${protocol}://localhost/${filePath}`,
  ),
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

const sampleScreenshot: MediaItem = {
  id: 1,
  path: "/screenshots/screen.png",
  filename: "screen.png",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-06-01T00:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  getScreenshots.mockResolvedValue([]);
  getScreenshotCount.mockResolvedValue(0);
});

describe("ScreenshotView", () => {
  it("shows loading state initially", () => {
    getScreenshots.mockReturnValue(new Promise(() => {}));
    getScreenshotCount.mockReturnValue(new Promise(() => {}));

    render(<ScreenshotView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("renders title and empty state", async () => {
    render(<ScreenshotView />);

    await waitFor(() => {
      expect(screen.getByText("截图")).toBeInTheDocument();
    });
    expect(screen.getByText("未发现截图")).toBeInTheDocument();
    expect(
      screen.getByText("扫描文件夹后，截图会自动识别"),
    ).toBeInTheDocument();
  });

  it("renders category filters", async () => {
    render(<ScreenshotView />);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "全部截图" }),
      ).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "代码" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "聊天" })).toBeInTheDocument();
  });

  it("loads screenshots and shows count", async () => {
    getScreenshots.mockResolvedValue([sampleScreenshot]);
    getScreenshotCount.mockResolvedValue(1);

    render(<ScreenshotView />);

    await waitFor(() => {
      expect(screen.getByText("共 1 项")).toBeInTheDocument();
    });
  });

  it("switches category and reloads data", async () => {
    const user = userEvent.setup();
    getScreenshotCount.mockResolvedValue(0);

    render(<ScreenshotView />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "代码" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "代码" }));

    await waitFor(() => {
      expect(getScreenshots).toHaveBeenCalledWith("code", 0, 60);
      expect(getScreenshotCount).toHaveBeenCalledWith("code");
    });
  });

  it("handles load errors gracefully", async () => {
    getScreenshots.mockRejectedValue(new Error("fail"));
    getScreenshotCount.mockRejectedValue(new Error("fail"));

    render(<ScreenshotView />);

    await waitFor(() => {
      expect(screen.getByText("未发现截图")).toBeInTheDocument();
    });
  });
});
