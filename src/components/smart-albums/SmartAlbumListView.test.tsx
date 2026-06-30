import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SmartAlbumListView } from "./SmartAlbumListView";
import { setLocale } from "@/i18n/index";
import * as appStore from "@/store/appStore";

const listSmartAlbums = vi.fn();
const createSmartAlbum = vi.fn();
const deleteSmartAlbum = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listSmartAlbums: (...args: unknown[]) => listSmartAlbums(...args),
    createSmartAlbum: (...args: unknown[]) => createSmartAlbum(...args),
    deleteSmartAlbum: (...args: unknown[]) => deleteSmartAlbum(...args),
  };
});

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  listSmartAlbums.mockResolvedValue([]);
});

describe("SmartAlbumListView", () => {
  it("shows loading state initially", () => {
    listSmartAlbums.mockReturnValue(new Promise(() => {}));

    render(<SmartAlbumListView />);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("shows empty state when no smart albums", async () => {
    render(<SmartAlbumListView />);

    await waitFor(() => {
      expect(screen.getByText("暂无智能相簿")).toBeInTheDocument();
    });
  });

  it("renders smart album list", async () => {
    listSmartAlbums.mockResolvedValue([
      {
        id: 1,
        name: "Landscapes",
        icon: "🏔️",
        media_count: 5,
        rule: { media_type: "Photo" },
        created_at: "2024-01-01",
      },
    ]);

    render(<SmartAlbumListView />);

    await waitFor(() => {
      expect(screen.getByText("Landscapes")).toBeInTheDocument();
    });
    expect(screen.getByText("共 5 项")).toBeInTheDocument();
  });

  it("opens create form and submits new album", async () => {
    const user = userEvent.setup();
    createSmartAlbum.mockResolvedValue(undefined);
    listSmartAlbums.mockResolvedValueOnce([]).mockResolvedValueOnce([
      {
        id: 2,
        name: "Portraits",
        icon: "✨",
        media_count: 0,
        rule: { media_type: "Photo" },
        created_at: "2024-06-01",
      },
    ]);

    render(<SmartAlbumListView />);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "创建智能相簿" }),
      ).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "创建智能相簿" }));
    await user.type(screen.getByRole("textbox"), "Portraits");
    const createButtons = screen.getAllByRole("button", {
      name: "创建智能相簿",
    });
    await user.click(createButtons[createButtons.length - 1]);

    await waitFor(() => {
      expect(createSmartAlbum).toHaveBeenCalledWith("Portraits", "✨", {
        media_type: "Photo",
      });
    });
  });

  it("opens smart album detail on click", async () => {
    const user = userEvent.setup();
    const openSpy = vi.spyOn(appStore, "openSmartAlbumDetail");
    listSmartAlbums.mockResolvedValue([
      {
        id: 3,
        name: "Videos",
        icon: "🎬",
        media_count: 2,
        rule: { media_type: "Video" },
        created_at: "2024-01-01",
      },
    ]);

    render(<SmartAlbumListView />);

    await waitFor(() => {
      expect(screen.getByText("Videos")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Videos"));
    expect(openSpy).toHaveBeenCalledWith(3);
    openSpy.mockRestore();
  });

  it("deletes smart album after confirmation", async () => {
    const user = userEvent.setup();
    vi.stubGlobal(
      "confirm",
      vi.fn(() => true),
    );
    deleteSmartAlbum.mockResolvedValue(undefined);
    listSmartAlbums.mockResolvedValue([
      {
        id: 4,
        name: "To Delete",
        icon: "📂",
        media_count: 1,
        rule: { media_type: "Photo" },
        created_at: "2024-01-01",
      },
    ]);

    render(<SmartAlbumListView />);

    await waitFor(() => {
      expect(screen.getByText("To Delete")).toBeInTheDocument();
    });

    await user.click(screen.getByTitle("删除相簿"));

    await waitFor(() => {
      expect(deleteSmartAlbum).toHaveBeenCalledWith(4);
    });
  });
});
