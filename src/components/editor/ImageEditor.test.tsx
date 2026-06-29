import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
}));

import { ImageEditor } from "./ImageEditor";

const defaultProps = {
  mediaId: 1,
  imagePath: "/photos/test.jpg",
  filename: "test.jpg",
  width: 1920,
  height: 1080,
  onClose: vi.fn(),
  onSaved: vi.fn(),
};

describe("ImageEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setLocale("zh-CN");
    vi.useFakeTimers({ shouldAdvanceTime: true });

    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_edit") return Promise.resolve(null);
      if (cmd === "save_edit") return Promise.resolve(undefined);
      if (cmd === "revert_edit") return Promise.resolve(undefined);
      if (cmd === "export_edited") return Promise.resolve(undefined);
      return Promise.resolve(null);
    });

    (save as ReturnType<typeof vi.fn>).mockResolvedValue("/output/test_edited.jpg");
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders editor UI", async () => {
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "编辑" })).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /撤销/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /重做/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "对比原图" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存" })).toBeInTheDocument();
    expect(screen.getByAltText("test.jpg")).toBeInTheDocument();
  });

  it("undo and redo functionality", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const undoBtn = screen.getByRole("button", { name: /撤销/ });
    const redoBtn = screen.getByRole("button", { name: /重做/ });
    expect(undoBtn).toBeDisabled();
    expect(redoBtn).toBeDisabled();

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "25" } });
    await vi.advanceTimersByTimeAsync(350);

    await waitFor(() => {
      expect(undoBtn).not.toBeDisabled();
    });
    expect(screen.getByText(/2\/2/)).toBeInTheDocument();

    await user.click(undoBtn);
    expect(undoBtn).toBeDisabled();
    expect(redoBtn).not.toBeDisabled();

    await user.click(redoBtn);
    expect(redoBtn).toBeDisabled();
    expect(undoBtn).not.toBeDisabled();
  });

  it("compare button shows original on pointer down", async () => {
    userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "50" } });
    await vi.advanceTimersByTimeAsync(350);

    const img = screen.getByAltText("test.jpg") as HTMLImageElement;
    const editedFilter = img.style.filter;

    const compareBtn = screen.getByRole("button", { name: "对比原图" });
    fireEvent.pointerDown(compareBtn);
    expect(img.style.filter).not.toBe(editedFilter);

    fireEvent.pointerUp(compareBtn);
    expect(img.style.filter).toBe(editedFilter);
  });

  it("reset resets to defaults", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "30" } });
    await vi.advanceTimersByTimeAsync(350);
    expect(brightnessSlider.value).toBe("30");

    await user.click(screen.getAllByRole("button", { name: "重置" })[0]!);
    expect(brightnessSlider.value).toBe("0");
  });

  it("save calls saveEdit when params are modified", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const onSaved = vi.fn();
    const onClose = vi.fn();

    render(<ImageEditor {...defaultProps} onSaved={onSaved} onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "15" } });
    await vi.advanceTimersByTimeAsync(350);

    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "save_edit",
        expect.objectContaining({ mediaId: 1 }),
      );
    });
    expect(onSaved).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("export calls save dialog and exportEdited", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "20" } });
    await vi.advanceTimersByTimeAsync(350);

    await user.click(screen.getByRole("button", { name: "导出" }));

    await waitFor(() => {
      expect(save).toHaveBeenCalledWith(
        expect.objectContaining({ defaultPath: "test_edited.jpg" }),
      );
    });
    expect(invoke).toHaveBeenCalledWith(
      "export_edited",
      expect.objectContaining({
        mediaId: 1,
        outputPath: "/output/test_edited.jpg",
        quality: 92,
      }),
    );
  });

  it("crop mode toggle via aspect ratio preset", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("裁剪")).toBeInTheDocument();
    });

    const img = screen.getByAltText("test.jpg");
    fireEvent.load(img);

    expect(document.querySelector(".cursor-move.border-2")).toBeNull();

    await user.click(screen.getByRole("button", { name: "正方形" }));

    await waitFor(() => {
      expect(document.querySelector(".cursor-move.border-2")).toBeInTheDocument();
    });
  });

  it("loads saved edit params from backend on mount", async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_edit") {
        return Promise.resolve(JSON.stringify({ exposure: 0.5, brightness: 10 }));
      }
      if (cmd === "save_edit") return Promise.resolve(undefined);
      if (cmd === "revert_edit") return Promise.resolve(undefined);
      return Promise.resolve(null);
    });

    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
      expect(brightnessSlider.value).toBe("10");
    });
  });

  it("save without modifications calls revertEdit", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("revert_edit", { mediaId: 1 });
      expect(invoke).not.toHaveBeenCalledWith("save_edit", expect.anything());
    });
  });

  it("shows save error when backend fails", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_edit") return Promise.resolve(null);
      if (cmd === "save_edit") return Promise.reject(new Error("disk full"));
      return Promise.resolve(null);
    });

    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "15" } });
    await vi.advanceTimersByTimeAsync(350);

    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(screen.getByText("保存编辑失败")).toBeInTheDocument();
    });
    consoleSpy.mockRestore();
  });

  it("Ctrl+Z triggers undo", async () => {
    render(<ImageEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const undoBtn = screen.getByRole("button", { name: /撤销/ });
    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "25" } });
    await vi.advanceTimersByTimeAsync(350);

    await waitFor(() => {
      expect(undoBtn).not.toBeDisabled();
    });

    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    expect(undoBtn).toBeDisabled();
  });

  it("revert resets edits and calls revertEdit", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const onSaved = vi.fn();
    render(<ImageEditor {...defaultProps} onSaved={onSaved} />);

    await waitFor(() => {
      expect(screen.getByText("亮度")).toBeInTheDocument();
    });

    const brightnessSlider = screen.getByLabelText("亮度") as HTMLInputElement;
    fireEvent.change(brightnessSlider, { target: { value: "20" } });
    await vi.advanceTimersByTimeAsync(350);

    await user.click(screen.getByRole("button", { name: "恢复原始" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("revert_edit", { mediaId: 1 });
      expect(brightnessSlider.value).toBe("0");
    });
    expect(onSaved).toHaveBeenCalled();
  });
});
