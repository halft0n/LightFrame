import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoriesView } from "./MemoriesView";
import { setLocale } from "@/i18n/index";

const listMemories = vi.fn();
const getOnThisDay = vi.fn();
const generateMemories = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listMemories: (...args: unknown[]) => listMemories(...args),
    getOnThisDay: (...args: unknown[]) => getOnThisDay(...args),
    generateMemories: (...args: unknown[]) => generateMemories(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  listMemories.mockReset();
  getOnThisDay.mockReset();
  generateMemories.mockReset();
});

describe("MemoriesView", () => {
  it("shows loading state initially", () => {
    listMemories.mockReturnValue(new Promise(() => {}));
    getOnThisDay.mockReturnValue(new Promise(() => {}));

    render(<MemoriesView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it('renders "On This Day" section when data is available', async () => {
    listMemories.mockResolvedValue([]);
    getOnThisDay.mockResolvedValue([
      {
        id: 10,
        path: "/photos/past.jpg",
        filename: "past.jpg",
        media_type: "Photo",
        size_bytes: 1024,
        modified_at: "2020-06-15T10:00:00",
      },
    ]);

    render(<MemoriesView />);

    await waitFor(() => {
      expect(screen.getByText("历史上的今天")).toBeInTheDocument();
    });
    expect(screen.getByText("往年今日的照片")).toBeInTheDocument();
    expect(screen.getByAltText("past.jpg")).toBeInTheDocument();
  });

  it("shows empty state when no memories exist", async () => {
    listMemories.mockResolvedValue([]);
    getOnThisDay.mockResolvedValue([]);

    render(<MemoriesView />);

    await waitFor(() => {
      expect(screen.getByText("暂无回忆")).toBeInTheDocument();
    });
    expect(
      screen.getByText("添加更多照片后，回忆将自动生成"),
    ).toBeInTheDocument();
  });

  it("renders memory cards when memories exist", async () => {
    listMemories.mockResolvedValue([
      {
        id: 1,
        title: "Summer 2024",
        subtitle: "Beach trip",
        cover_media_id: 5,
        media_count: 12,
        date_from: "2024-07-01",
        date_to: "2024-07-15",
        created_at: "2024-08-01T00:00:00",
      },
    ]);
    getOnThisDay.mockResolvedValue([]);

    render(<MemoriesView />);

    await waitFor(() => {
      expect(screen.getByText("Summer 2024")).toBeInTheDocument();
    });
    expect(screen.getByText("Beach trip")).toBeInTheDocument();
    expect(screen.getByText("12 张照片")).toBeInTheDocument();
  });

  it("calls generateMemories when generate button is clicked", async () => {
    const user = userEvent.setup();
    listMemories.mockResolvedValue([]);
    getOnThisDay.mockResolvedValue([]);
    generateMemories.mockResolvedValue([
      {
        id: 2,
        title: "New Memory",
        subtitle: null,
        cover_media_id: 1,
        media_count: 3,
        date_from: "2024-01-01",
        date_to: "2024-01-07",
        created_at: "2024-02-01T00:00:00",
      },
    ]);

    render(<MemoriesView />);

    await waitFor(() => {
      expect(screen.getByText("生成回忆")).toBeInTheDocument();
    });

    await user.click(screen.getByText("生成回忆"));

    await waitFor(() => {
      expect(generateMemories).toHaveBeenCalled();
      expect(screen.getByText("New Memory")).toBeInTheDocument();
    });
  });
});
