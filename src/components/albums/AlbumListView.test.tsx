import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AlbumListView } from "./AlbumListView";
import { setLocale } from "@/i18n/index";

const listAlbums = vi.fn();
const createAlbum = vi.fn();
const updateAlbum = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listAlbums: (...args: unknown[]) => listAlbums(...args),
    createAlbum: (...args: unknown[]) => createAlbum(...args),
    updateAlbum: (...args: unknown[]) => updateAlbum(...args),
    deleteAlbum: vi.fn(),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  listAlbums.mockResolvedValue([]);
});

describe("AlbumListView", () => {
  it("shows empty state when no albums", async () => {
    render(<AlbumListView />);
    await waitFor(() => {
      expect(screen.getByText("暂无相簿")).toBeInTheDocument();
    });
  });

  it("opens create album form and submits", async () => {
    const user = userEvent.setup();
    createAlbum.mockResolvedValue({
      id: 1,
      name: "Summer",
      media_count: 0,
      created_at: "2024-01-01",
    });
    listAlbums
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([
        { id: 1, name: "Summer", media_count: 0, created_at: "2024-01-01" },
      ]);

    render(<AlbumListView />);
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "新建相簿" }),
      ).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "新建相簿" }));
    const nameInput = screen.getByRole("textbox");
    await user.type(nameInput, "Summer");
    const createButtons = screen.getAllByRole("button", { name: "新建相簿" });
    await user.click(createButtons[createButtons.length - 1]);

    await waitFor(() => {
      expect(createAlbum).toHaveBeenCalledWith("Summer", null);
    });
  });

  it("renders album list and supports rename", async () => {
    const user = userEvent.setup();
    vi.clearAllMocks();
    listAlbums.mockResolvedValue([
      {
        id: 5,
        name: "Trip",
        description: null,
        media_count: 3,
        created_at: "2024-06-01",
      },
    ]);
    updateAlbum.mockResolvedValue(undefined);

    render(<AlbumListView />);
    await waitFor(() => {
      expect(screen.getByText("Trip")).toBeInTheDocument();
    });

    await user.click(screen.getByTitle("重命名相簿"));
    const input = screen.getByDisplayValue("Trip");
    await user.clear(input);
    await user.type(input, "Vacation");
    await user.click(screen.getByRole("button", { name: "重命名" }));

    await waitFor(() => {
      expect(updateAlbum).toHaveBeenCalledWith(5, "Vacation", null);
    });
  });
});
