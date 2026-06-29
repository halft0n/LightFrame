import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SimilarPhotosView } from "./SimilarPhotosView";
import { setLocale } from "@/i18n/index";

const findSimilarPhotos = vi.fn();
const computeClipEmbedding = vi.fn();
const openViewer = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    findSimilarPhotos: (...args: unknown[]) => findSimilarPhotos(...args),
    computeClipEmbedding: (...args: unknown[]) => computeClipEmbedding(...args),
  };
});

vi.mock("@/store/appStore", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/store/appStore")>();
  return {
    ...actual,
    openViewer: (...args: unknown[]) => openViewer(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  findSimilarPhotos.mockReset();
  computeClipEmbedding.mockReset();
  openViewer.mockReset();
});

describe("SimilarPhotosView", () => {
  it("renders loading state", () => {
    findSimilarPhotos.mockReturnValue(new Promise(() => {}));

    render(<SimilarPhotosView mediaId={1} />);
    expect(screen.getByText("计算中…")).toBeInTheDocument();
  });

  it("renders empty state when no similar photos", async () => {
    findSimilarPhotos.mockResolvedValue([]);

    render(<SimilarPhotosView mediaId={1} />);
    await waitFor(() => {
      expect(screen.getByText("未找到相似照片")).toBeInTheDocument();
    });
  });

  it("renders similar photos with similarity scores", async () => {
    findSimilarPhotos.mockResolvedValue([
      { media_id: 10, similarity: 0.87 },
      { media_id: 11, similarity: 0.62 },
    ]);

    render(<SimilarPhotosView mediaId={1} />);
    await waitFor(() => {
      expect(screen.getByLabelText("87% 相似")).toBeInTheDocument();
      expect(screen.getByLabelText("62% 相似")).toBeInTheDocument();
    });
  });

  it("shows error state when search fails", async () => {
    findSimilarPhotos.mockRejectedValue(new Error("CLIP unavailable"));

    render(<SimilarPhotosView mediaId={1} />);
    await waitFor(() => {
      expect(screen.getByText("操作失败，请重试")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
  });

  it("click on photo opens viewer", async () => {
    const user = userEvent.setup();
    findSimilarPhotos.mockResolvedValue([{ media_id: 42, similarity: 0.95 }]);

    render(<SimilarPhotosView mediaId={1} />);
    await waitFor(() => {
      expect(screen.getByLabelText("95% 相似")).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText("95% 相似"));
    expect(openViewer).toHaveBeenCalledWith(42);
  });
});
