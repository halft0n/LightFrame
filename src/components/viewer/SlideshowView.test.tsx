import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SlideshowView } from "./SlideshowView";
import { setLocale } from "@/i18n/index";
import {
  closeSlideshow,
  getSnapshot,
  startSlideshow,
} from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

const getMediaById = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getMediaById: (...args: unknown[]) => getMediaById(...args),
  };
});

const mockPhoto: MediaItem = {
  id: 1,
  path: "/photos/a.jpg",
  filename: "a.jpg",
  media_type: "Photo",
  size_bytes: 1024,
  modified_at: "2024-01-01T00:00:00",
};

const mockPhoto2: MediaItem = {
  ...mockPhoto,
  id: 2,
  filename: "b.jpg",
  path: "/photos/b.jpg",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  closeSlideshow();
  vi.clearAllMocks();
  getMediaById.mockImplementation((id: number) => {
    if (id === 2) return Promise.resolve(mockPhoto2);
    return Promise.resolve(mockPhoto);
  });
});

describe("SlideshowView", () => {
  it("shows empty state when no photos", () => {
    render(<SlideshowView />);

    expect(screen.getByText("没有可播放的照片")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "退出" })).toBeInTheDocument();
  });

  it("closes slideshow from empty state", async () => {
    const user = userEvent.setup();
    render(<SlideshowView />);

    await user.click(screen.getByRole("button", { name: "退出" }));
    expect(getSnapshot().slideshowActive).toBe(false);
  });

  it("loads and displays current slide", async () => {
    startSlideshow([1, 2], 1);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByAltText("a.jpg")).toBeInTheDocument();
    });
    expect(screen.getByText("1 / 2")).toBeInTheDocument();
  });

  it("shows loading while media is fetched", () => {
    getMediaById.mockReturnValue(new Promise(() => {}));
    startSlideshow([1]);
    render(<SlideshowView />);

    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("navigates with next and previous buttons", async () => {
    const user = userEvent.setup();
    startSlideshow([1, 2]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByAltText("a.jpg")).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText("下一张"));

    await waitFor(() => {
      expect(screen.getByAltText("b.jpg")).toBeInTheDocument();
      expect(screen.getByText("2 / 2")).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText("上一张"));

    await waitFor(() => {
      expect(screen.getByAltText("a.jpg")).toBeInTheDocument();
    });
  });

  it("responds to arrow key navigation", async () => {
    startSlideshow([1, 2]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByAltText("a.jpg")).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => {
      expect(getSnapshot().slideshowIndex).toBe(1);
    });

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => {
      expect(getSnapshot().slideshowIndex).toBe(0);
    });
  });

  it("toggles play/pause with space key", async () => {
    startSlideshow([1]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByLabelText("暂停")).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: " ", code: "Space" });
    expect(screen.getByLabelText("播放")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: " ", code: "Space" });
    expect(screen.getByLabelText("暂停")).toBeInTheDocument();
  });

  it("closes slideshow on Escape", async () => {
    startSlideshow([1]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "Escape" });
    expect(getSnapshot().slideshowActive).toBe(false);
  });

  it("changes slideshow speed", async () => {
    const user = userEvent.setup();
    startSlideshow([1]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByText("3秒")).toBeInTheDocument();
    });

    await user.click(screen.getByText("10秒"));
    expect(getSnapshot().slideshowSpeed).toBe(10);
  });

  it("hides nav buttons for single photo", async () => {
    startSlideshow([1]);
    render(<SlideshowView />);

    await waitFor(() => {
      expect(screen.getByAltText("a.jpg")).toBeInTheDocument();
    });

    expect(screen.queryByLabelText("上一张")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("下一张")).not.toBeInTheDocument();
  });

});
