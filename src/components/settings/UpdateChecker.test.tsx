import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { UpdateChecker } from "./UpdateChecker";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue("0.0.16"),
  convertFileSrc: vi.fn(
    (filePath: string, protocol: string = "asset") =>
      `${protocol}://localhost/${filePath}`,
  ),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn().mockResolvedValue(undefined),
}));

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
});

describe("UpdateChecker", () => {
  it("renders title and check button", () => {
    render(<UpdateChecker />);
    expect(screen.getByText("检查更新")).toBeInTheDocument();
  });

  it("shows checking state while fetching version", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(
        new Response(JSON.stringify({ tag_name: "v0.0.16" }), { status: 200 }),
      );
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText(/v0\.0\.16/)).toBeInTheDocument();
    });

    fetchSpy.mockRestore();
  });

  it("shows up-to-date when versions match", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ tag_name: "v0.0.16" }), { status: 200 }),
    );
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("当前已是最新版本")).toBeInTheDocument();
    });
  });

  it("shows error state when fetch fails", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("network"));
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("检查更新失败")).toBeInTheDocument();
    });
  });

  it("shows new version available with link", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ tag_name: "v0.0.17" }), { status: 200 }),
    );
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText(/v0\.0\.17/)).toBeInTheDocument();
    });
  });
});
