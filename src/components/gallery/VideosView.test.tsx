import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { VideosView } from "./VideosView";
import { setLocale } from "@/i18n/index";
import type { MediaItem } from "@/lib/tauri";

const getMediaByType = vi.fn();
const getMediaCountByType = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getMediaByType: (...args: unknown[]) => getMediaByType(...args),
    getMediaCountByType: (...args: unknown[]) => getMediaCountByType(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

const sampleVideo: MediaItem = {
  id: 1,
  path: "/videos/clip.mp4",
  filename: "clip.mp4",
  media_type: "Video",
  size_bytes: 4096,
  modified_at: "2024-01-01T00:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getMediaByType.mockReset();
  getMediaCountByType.mockReset();
});

describe("VideosView", () => {
  it("shows loading state initially", () => {
    getMediaByType.mockReturnValue(new Promise(() => {}));
    getMediaCountByType.mockReturnValue(new Promise(() => {}));

    render(<VideosView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it('shows "no videos" message when empty', async () => {
    getMediaByType.mockResolvedValue([]);
    getMediaCountByType.mockResolvedValue(0);

    render(<VideosView />);
    await waitFor(() => {
      expect(screen.getByText("暂无视频")).toBeInTheDocument();
    });
  });

  it("loads videos by type", async () => {
    getMediaByType.mockResolvedValue([sampleVideo]);
    getMediaCountByType.mockResolvedValue(1);

    render(<VideosView />);
    await waitFor(() => {
      expect(getMediaByType).toHaveBeenCalledWith("Video", 0, 60);
      expect(getMediaCountByType).toHaveBeenCalledWith("Video");
    });
  });
});
