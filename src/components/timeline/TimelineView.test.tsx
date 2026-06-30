import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { setLocale } from "@/i18n/index";
import { getTimelineGroups } from "@/lib/tauri";

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getTimelineGroups: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  convertFileSrc: vi.fn((path: string) => `file://${path}`),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

class ResizeObserverMock {
  observe = vi.fn((target: Element) => {
    this.callback?.(
      [
        {
          contentRect: { width: 800, height: 600 },
          target,
        } as ResizeObserverEntry,
      ],
      this as unknown as ResizeObserver,
    );
  });
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor(private callback?: ResizeObserverCallback) {}
}
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

import { TimelineView } from "./TimelineView";

const mockGroups = [
  {
    date: "2024-06-15",
    count: 2,
    media: [
      {
        id: 1,
        path: "/photos/a.jpg",
        filename: "a.jpg",
        media_type: "Photo" as const,
        size_bytes: 1024,
        modified_at: "2024-06-15T10:00:00",
      },
      {
        id: 2,
        path: "/photos/b.jpg",
        filename: "b.jpg",
        media_type: "Photo" as const,
        size_bytes: 1024,
        modified_at: "2024-06-15T14:00:00",
      },
    ],
  },
  {
    date: "2024-06-14",
    count: 1,
    media: [
      {
        id: 3,
        path: "/photos/c.jpg",
        filename: "c.jpg",
        media_type: "Photo" as const,
        size_bytes: 1024,
        modified_at: "2024-06-14T09:00:00",
      },
    ],
  },
];

describe("TimelineView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setLocale("zh-CN");
    (getTimelineGroups as ReturnType<typeof vi.fn>).mockResolvedValue(
      mockGroups,
    );
  });

  it("shows loading state initially", () => {
    (getTimelineGroups as ReturnType<typeof vi.fn>).mockImplementation(
      () => new Promise(() => {}),
    );
    render(<TimelineView />);
    expect(
      screen.getByText(/加载中|正在加载照片|Loading/i),
    ).toBeInTheDocument();
  });

  it("renders timeline summary after groups load", async () => {
    render(<TimelineView />);

    expect(await screen.findByText(/共 3 项/)).toBeInTheDocument();
    expect(screen.getByText(/时间线/)).toBeInTheDocument();
    expect(getTimelineGroups).toHaveBeenCalledWith(200);
  });
});
