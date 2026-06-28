import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `file://${path}`),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { PhotoViewer } from "./PhotoViewer";

const mockMedia = {
  id: 1,
  path: "/photos/test.jpg",
  filename: "test.jpg",
  media_type: "Photo",
  size_bytes: 1024000,
  width: 1920,
  height: 1080,
  created_at: "2024-06-15T10:00:00",
  modified_at: "2024-06-15T10:00:00",
};

describe("PhotoViewer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setLocale("zh-CN");
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_media_by_id") return Promise.resolve(mockMedia);
      if (cmd === "get_media_neighbors")
        return Promise.resolve({ prev_id: null, next_id: null });
      if (cmd === "get_media_list") return Promise.resolve([mockMedia]);
      if (cmd === "has_edits") return Promise.resolve(false);
      if (cmd === "get_edit") return Promise.resolve(null);
      return Promise.resolve(null);
    });
  });

  it("renders close button", () => {
    render(<PhotoViewer mediaId={1} />);
    const closeBtn = screen.getByLabelText(/关闭|Close/i);
    expect(closeBtn).toBeInTheDocument();
  });

  it("renders zoom controls", () => {
    render(<PhotoViewer mediaId={1} />);
    expect(screen.getByTitle(/放大|Zoom in/i)).toBeInTheDocument();
    expect(screen.getByTitle(/缩小|Zoom out/i)).toBeInTheDocument();
  });
});
