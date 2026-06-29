import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { UpdateChecker } from "./UpdateChecker";
import { setLocale } from "@/i18n/index";

const check = vi.fn();
const relaunch = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: () => check(),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: () => relaunch(),
}));

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  relaunch.mockResolvedValue(undefined);
});

describe("UpdateChecker", () => {
  it("shows up-to-date message when no update available", async () => {
    check.mockResolvedValue(null);
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("当前已是最新版本")).toBeInTheDocument();
    });
  });

  it("shows downloading notification when update is available", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    check.mockResolvedValue({
      version: "2.0.0",
      downloadAndInstall,
    });
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getAllByText(/正在下载 v2\.0\.0/).length).toBeGreaterThan(0);
    });
    expect(downloadAndInstall).toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalled();
  });

  it("shows error message when check fails", async () => {
    check.mockRejectedValue(new Error("network timeout"));
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText("network timeout")).toBeInTheDocument();
    });
  });

  it("shows tauri-only message outside Tauri environment", async () => {
    delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    const user = userEvent.setup();

    render(<UpdateChecker />);
    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByText(/此功能仅在桌面应用中可用/)).toBeInTheDocument();
    });
    expect(check).not.toHaveBeenCalled();
  });
});
