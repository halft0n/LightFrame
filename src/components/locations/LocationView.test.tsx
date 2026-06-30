import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { LocationView } from "./LocationView";
import { setLocale } from "@/i18n/index";
import type { LocationStats } from "@/lib/tauri";

const getLocationGroups = vi.fn();
const getLocationStats = vi.fn();

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
vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getLocationGroups: (...args: unknown[]) => getLocationGroups(...args),
    getLocationStats: (...args: unknown[]) => getLocationStats(...args),
  };
});

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
globalThis.ResizeObserver =
  ResizeObserverMock as unknown as typeof ResizeObserver;

const emptyStats: LocationStats = {
  total_with_gps: 0,
  countries: 0,
  cities: 0,
};

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getLocationGroups.mockReset();
  getLocationStats.mockReset();
});

describe("LocationView", () => {
  it("shows loading state initially", () => {
    getLocationGroups.mockReturnValue(new Promise(() => {}));
    getLocationStats.mockReturnValue(new Promise(() => {}));

    render(<LocationView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("shows empty state when no locations", async () => {
    getLocationGroups.mockResolvedValue([]);
    getLocationStats.mockResolvedValue(emptyStats);

    render(<LocationView />);
    await waitFor(() => {
      expect(screen.getByText("暂无位置信息")).toBeInTheDocument();
    });
    expect(screen.getByText("拍摄时开启GPS即可按地点浏览")).toBeInTheDocument();
  });
});
