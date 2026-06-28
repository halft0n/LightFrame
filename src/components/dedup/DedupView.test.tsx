import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { DedupView } from "./DedupView";
import { setLocale } from "@/i18n/index";

const getDuplicateGroups = vi.fn();
const runDedupScan = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getDuplicateGroups: (...args: unknown[]) => getDuplicateGroups(...args),
    runDedupScan: (...args: unknown[]) => runDedupScan(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  getDuplicateGroups.mockReset();
  runDedupScan.mockReset();
});

describe("DedupView", () => {
  it("shows loading state initially", () => {
    getDuplicateGroups.mockReturnValue(new Promise(() => {}));

    render(<DedupView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("shows empty state when no duplicate groups", async () => {
    getDuplicateGroups.mockResolvedValue([]);

    render(<DedupView />);
    await waitFor(() => {
      expect(screen.getByText("未发现重复照片")).toBeInTheDocument();
    });
    expect(screen.getByText("所有照片都是唯一的")).toBeInTheDocument();
  });

  it("renders dedup title and scan button after loading", async () => {
    getDuplicateGroups.mockResolvedValue([]);

    render(<DedupView />);
    await waitFor(() => {
      expect(screen.getByText("重复照片")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "扫描重复" })).toBeInTheDocument();
  });
});
