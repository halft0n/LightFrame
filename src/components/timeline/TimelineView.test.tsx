import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
  convertFileSrc: vi.fn((path: string) => `file://${path}`),
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

import { TimelineView } from "./TimelineView";

describe("TimelineView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setLocale("zh-CN");
  });

  it("shows loading state initially", () => {
    render(<TimelineView />);
    expect(screen.getByText(/加载中|Loading/i)).toBeInTheDocument();
  });
});
