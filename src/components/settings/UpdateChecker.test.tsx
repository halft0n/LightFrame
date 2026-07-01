import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { UpdateChecker } from "./UpdateChecker";
import { setLocale } from "@/i18n/index";

const checkForUpdatesMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn(
    (filePath: string, protocol: string = "asset") =>
      `${protocol}://localhost/${filePath}`,
  ),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    checkForUpdates: () => checkForUpdatesMock(),
  };
});

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

  it("shows up-to-date when versions match", async () => {
    checkForUpdatesMock.mockResolvedValue({
      current_version: "0.0.17",
      latest_version: "0.0.17",
      update_available: false,
      release_url: "https://github.com/halft0n/LightFrame/releases/tag/v0.0.17",
    });
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("当前已是最新版本")).toBeInTheDocument();
    });
  });

  it("shows error state when check fails", async () => {
    checkForUpdatesMock.mockRejectedValue(new Error("network error"));
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("network error")).toBeInTheDocument();
    });
  });

  it("shows new version available with link", async () => {
    checkForUpdatesMock.mockResolvedValue({
      current_version: "0.0.16",
      latest_version: "0.0.17",
      update_available: true,
      release_url: "https://github.com/halft0n/LightFrame/releases/tag/v0.0.17",
    });
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText(/v0\.0\.17/)).toBeInTheDocument();
      expect(screen.getByText("查看版本")).toBeInTheDocument();
    });
  });

  it("shows current version after check", async () => {
    checkForUpdatesMock.mockResolvedValue({
      current_version: "0.0.17",
      latest_version: "0.0.17",
      update_available: false,
      release_url: "https://github.com/halft0n/LightFrame/releases/tag/v0.0.17",
    });
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText(/v0\.0\.17/)).toBeInTheDocument();
    });
  });
});
