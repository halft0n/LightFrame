import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { setLocale } from "@/i18n/index";
import * as appStore from "@/store/appStore";

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

import { PhotoViewer } from "./PhotoViewer";

const mockPhoto = {
  id: 1,
  path: "/photos/test.jpg",
  filename: "test.jpg",
  media_type: "Photo" as const,
  size_bytes: 1024000,
  width: 1920,
  height: 1080,
  created_at: "2024-06-15T10:00:00",
  modified_at: "2024-06-15T10:00:00",
};

const mockVideo = {
  ...mockPhoto,
  id: 2,
  path: "/videos/clip.mp4",
  filename: "clip.mp4",
  media_type: "Video" as const,
  duration_sec: 60,
};

function getMainImage() {
  return document.querySelector("img.select-none");
}

function setupInvoke(
  options: {
    isFavorite?: boolean;
    neighbors?: { prev_id: number | null; next_id: number | null };
  } = {},
) {
  (invoke as ReturnType<typeof vi.fn>).mockImplementation(
    (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_media_by_id") {
        const id = (args?.id as number | undefined) ?? 1;
        if (id === mockVideo.id) return Promise.resolve(mockVideo);
        return Promise.resolve({ ...mockPhoto, id });
      }
      if (cmd === "get_media_neighbors") {
        return Promise.resolve(
          options.neighbors ?? { prev_id: 10, next_id: 20 },
        );
      }
      if (cmd === "get_media_window") {
        const mediaId = (args?.mediaId as number | undefined) ?? 1;
        return Promise.resolve([{ ...mockPhoto, id: mediaId }]);
      }
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "get_edit") return Promise.resolve(null);
      if (cmd === "is_favorite")
        return Promise.resolve(options.isFavorite ?? false);
      if (cmd === "toggle_favorite")
        return Promise.resolve(!(options.isFavorite ?? false));
      return Promise.resolve(null);
    },
  );
}

describe("PhotoViewer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setLocale("zh-CN");
    setupInvoke();
  });

  it("renders loading state before media loads", () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_by_id") return new Promise(() => {});
      if (cmd === "get_media_neighbors") return new Promise(() => {});
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "is_favorite") return Promise.resolve(false);
      return Promise.resolve(null);
    });

    render(<PhotoViewer mediaId={1} />);
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });

  it("loads media data and shows image", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });
    expect(getMainImage()).toHaveAttribute("src", "thumb://localhost/1/large");
  });

  it("shows favorite button with unfavorited state", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toBeInTheDocument();
    });
    expect(screen.getByLabelText("收藏")).toHaveTextContent("♡");
  });

  it("shows favorite button with favorited state", async () => {
    setupInvoke({ isFavorite: true });
    (invoke as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string, _args?: Record<string, unknown>) => {
        if (cmd === "get_media_by_id") return Promise.resolve(mockPhoto);
        if (cmd === "get_media_neighbors")
          return Promise.resolve({ prev_id: null, next_id: null });
        if (cmd === "get_media_window") return Promise.resolve([mockPhoto]);
        if (cmd === "has_edits") return Promise.resolve(false);
        if (cmd === "is_favorite") return Promise.resolve(true);
        if (cmd === "toggle_favorite") return Promise.resolve(false);
        return Promise.resolve(null);
      },
    );

    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toHaveTextContent("♥");
    });
  });

  it("toggle favorite updates state", async () => {
    const user = userEvent.setup();
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toHaveTextContent("♡");
    });

    await user.click(screen.getByLabelText("收藏"));

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toHaveTextContent("♥");
    });
    expect(invoke).toHaveBeenCalledWith("toggle_favorite", { mediaId: 1 });
  });

  it("navigates with arrow keys for photos", async () => {
    const openViewerSpy = vi.spyOn(appStore, "openViewer");
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    expect(openViewerSpy).toHaveBeenCalledWith(10);

    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(openViewerSpy).toHaveBeenCalledWith(20);

    openViewerSpy.mockRestore();
  });

  it("zoom controls change zoom level", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("缩放")).toBeInTheDocument();
    });

    const slider = screen.getByLabelText("缩放") as HTMLInputElement;
    expect(slider.value).toBe("1");

    fireEvent.change(slider, { target: { value: "2.5" } });
    expect(slider.value).toBe("2.5");
  });

  it("info panel toggle shows and hides metadata", async () => {
    const user = userEvent.setup();
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    expect(screen.queryByText("文件名")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "信息" }));
    expect(screen.getByText("文件名")).toBeInTheDocument();
    expect(
      screen.getByRole("complementary").querySelector("dd"),
    ).toHaveTextContent("test.jpg");
    expect(screen.getByText("1920 × 1080")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "信息" }));
    expect(screen.queryByText("文件名")).not.toBeInTheDocument();
  });

  it("editor open and close", async () => {
    const user = userEvent.setup();
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "编辑" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "编辑" }));
    expect(screen.getByRole("heading", { name: "编辑" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "取消" }));
    expect(
      screen.queryByRole("heading", { name: "编辑" }),
    ).not.toBeInTheDocument();
  });

  it("renders video player for video media", async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_by_id") return Promise.resolve(mockVideo);
      if (cmd === "get_media_neighbors")
        return Promise.resolve({ prev_id: null, next_id: null });
      if (cmd === "get_media_window") return Promise.resolve([mockVideo]);
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "is_favorite") return Promise.resolve(false);
      return Promise.resolve(null);
    });

    render(<PhotoViewer mediaId={2} />);

    await waitFor(() => {
      expect(screen.getByLabelText("播放")).toBeInTheDocument();
    });
    expect(screen.queryByLabelText("缩放")).not.toBeInTheDocument();
    expect(document.querySelector("video")).toBeInTheDocument();
  });

  it("renders back button", async () => {
    render(<PhotoViewer mediaId={1} />);
    await waitFor(() => {
      expect(screen.getByLabelText("返回")).toBeInTheDocument();
    });
  });

  it("rotates clockwise with R key", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "r" });
    expect(screen.getByText("90°")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "R" });
    expect(screen.getByText("180°")).toBeInTheDocument();
  });

  it("shows print and copy buttons for photos", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("打印")).toBeInTheDocument();
      expect(screen.getByLabelText("复制图片")).toBeInTheDocument();
    });
  });

  it("calls window.print when print button clicked", async () => {
    const printSpy = vi.spyOn(window, "print").mockImplementation(() => {});
    const user = userEvent.setup();
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("打印")).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText("打印"));
    expect(printSpy).toHaveBeenCalled();
    printSpy.mockRestore();
  });

  it("copies image to clipboard when copy button clicked", async () => {
    vi.stubGlobal(
      "ClipboardItem",
      class {
        constructor(public items: Record<string, Blob>) {}
      },
    );
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        blob: () => Promise.resolve(new Blob(["img"], { type: "image/jpeg" })),
      }),
    );

    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: {
        write: vi.fn().mockResolvedValue(undefined),
      },
    });

    const user = userEvent.setup();
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("复制图片")).toBeEnabled();
    });

    await user.click(screen.getByLabelText("复制图片"));

    await waitFor(() => {
      expect(screen.getByLabelText("复制图片")).toHaveAttribute(
        "title",
        "图片已复制到剪贴板",
      );
    });
  });

  it("rotates counter-clockwise with Shift+R", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "r" });
    fireEvent.keyDown(window, { key: "r" });
    expect(screen.getByText("180°")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "R", shiftKey: true });
    expect(screen.getByText("90°")).toBeInTheDocument();
  });

  it("toggles favorite with F key", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toHaveTextContent("♡");
    });

    fireEvent.keyDown(window, { key: "f" });

    await waitFor(() => {
      expect(screen.getByLabelText("收藏")).toHaveTextContent("♥");
    });
    expect(invoke).toHaveBeenCalledWith("toggle_favorite", { mediaId: 1 });
  });

  it("toggles info panel with I key", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    expect(screen.queryByText("文件名")).not.toBeInTheDocument();

    fireEvent.keyDown(window, { key: "i" });
    expect(screen.getByText("文件名")).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "I" });
    expect(screen.queryByText("文件名")).not.toBeInTheDocument();
  });

  it("opens editor with E key", async () => {
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "e" });
    expect(screen.getByRole("heading", { name: "编辑" })).toBeInTheDocument();
  });

  it("soft-deletes media with Delete key", async () => {
    vi.stubGlobal(
      "confirm",
      vi.fn(() => true),
    );
    const closeViewerSpy = vi.spyOn(appStore, "closeViewer");
    const loadMediaSpy = vi
      .spyOn(appStore, "loadMedia")
      .mockResolvedValue(undefined);
    (invoke as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string, _args?: Record<string, unknown>) => {
        if (cmd === "get_media_by_id") return Promise.resolve(mockPhoto);
        if (cmd === "get_media_neighbors")
          return Promise.resolve({ prev_id: null, next_id: null });
        if (cmd === "get_media_window") return Promise.resolve([mockPhoto]);
        if (cmd === "has_edits") return Promise.resolve(false);
        if (cmd === "is_favorite") return Promise.resolve(false);
        if (cmd === "delete_media") return Promise.resolve(undefined);
        return Promise.resolve(null);
      },
    );

    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "Delete" });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("delete_media", { mediaId: 1 });
      expect(closeViewerSpy).toHaveBeenCalled();
      expect(loadMediaSpy).toHaveBeenCalled();
    });

    closeViewerSpy.mockRestore();
    loadMediaSpy.mockRestore();
  });

  it("preloads adjacent images after main image loads", async () => {
    const createdImages: Array<{ src: string; onload: (() => void) | null }> =
      [];
    const OriginalImage = globalThis.Image;

    class MockImageClass {
      src = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;

      constructor() {
        createdImages.push(this);
      }
    }

    vi.stubGlobal("Image", MockImageClass);

    try {
      render(<PhotoViewer mediaId={1} />);

      await waitFor(() => {
        expect(getMainImage()).toBeInTheDocument();
      });

      fireEvent.load(getMainImage()!);

      await waitFor(() => {
        expect(
          createdImages.some((img) => img.src.startsWith("original://")),
        ).toBe(true);
      });
    } finally {
      vi.stubGlobal("Image", OriginalImage);
    }
  });

  it("closes viewer on Escape after exit animation", async () => {
    const closeViewerSpy = vi.spyOn(appStore, "closeViewer");
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    vi.useFakeTimers();
    try {
      fireEvent.keyDown(window, { key: "Escape" });
      expect(closeViewerSpy).not.toHaveBeenCalled();
      vi.advanceTimersByTime(250);
      expect(closeViewerSpy).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
      closeViewerSpy.mockRestore();
    }
  });

  it("does not navigate when neighbors are null", async () => {
    const openViewerSpy = vi.spyOn(appStore, "openViewer");
    setupInvoke({ neighbors: { prev_id: null, next_id: null } });
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_by_id") return Promise.resolve(mockPhoto);
      if (cmd === "get_media_neighbors") {
        return Promise.resolve({ prev_id: null, next_id: null });
      }
      if (cmd === "get_media_window") return Promise.resolve([mockPhoto]);
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "is_favorite") return Promise.resolve(false);
      return Promise.resolve(null);
    });

    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(openViewerSpy).not.toHaveBeenCalled();
    openViewerSpy.mockRestore();
  });

  it("back button triggers close animation", async () => {
    const closeViewerSpy = vi.spyOn(appStore, "closeViewer");
    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByLabelText("返回")).toBeInTheDocument();
    });

    vi.useFakeTimers();
    try {
      fireEvent.click(screen.getByLabelText("返回"));
      vi.advanceTimersByTime(250);
      expect(closeViewerSpy).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
      closeViewerSpy.mockRestore();
    }
  });

  it("shows error state on load failure and retries on click", async () => {
    let callCount = 0;
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_by_id") {
        callCount++;
        if (callCount <= 1) return Promise.reject(new Error("fail"));
        return Promise.resolve(mockPhoto);
      }
      if (cmd === "get_media_neighbors")
        return Promise.resolve({ prev_id: null, next_id: null });
      if (cmd === "get_media_window") return Promise.resolve([mockPhoto]);
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "is_favorite") return Promise.resolve(false);
      return Promise.resolve(null);
    });

    render(<PhotoViewer mediaId={1} />);

    await waitFor(() => {
      expect(screen.getByText("加载媒体失败")).toBeInTheDocument();
    });
    expect(screen.getByText("重试")).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByText("重试"));

    await waitFor(() => {
      expect(getMainImage()).toBeInTheDocument();
    });
  });
});
