import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { PhotoGrid } from "./PhotoGrid";
import { setLocale } from "@/i18n/index";
import { setMedia, getSnapshot } from "@/store/appStore";
import type { MediaItem } from "@/lib/tauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

const sampleMedia: MediaItem = {
  id: 1,
  path: "/photos/sunset.jpg",
  filename: "sunset.jpg",
  media_type: "Photo",
  size_bytes: 2048,
  modified_at: "2024-01-01T00:00:00",
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  setMedia([], 0);
});

describe("PhotoGrid", () => {
  it('shows "no photos" message when empty', () => {
    render(<PhotoGrid />);
    expect(screen.getByText("暂无照片")).toBeInTheDocument();
  });

  it("renders gallery count header", () => {
    setMedia([sampleMedia], 15);
    render(<PhotoGrid />);
    expect(screen.getByText("共 15 项")).toBeInTheDocument();
    expect(getSnapshot().mediaItems).toHaveLength(1);
  });
});
