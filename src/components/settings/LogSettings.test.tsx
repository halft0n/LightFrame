import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { LogSettings } from "./LogSettings";
import { setLocale } from "@/i18n/index";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const defaultConfig = {
  level: "debug",
  retention_days: 14,
  max_size_mb: 200,
};

const sampleLogFiles = [
  { path: "/logs/app.log", size_bytes: 1_048_576, modified: "2024-06-01T00:00:00" },
  { path: "/logs/app.old.log", size_bytes: 524_288, modified: "2024-05-01T00:00:00" },
];

function setupInvoke(overrides: {
  config?: typeof defaultConfig;
  logDir?: string;
  logFiles?: typeof sampleLogFiles;
} = {}) {
  (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
    if (cmd === "get_log_config") return Promise.resolve(overrides.config ?? defaultConfig);
    if (cmd === "get_log_directory") return Promise.resolve(overrides.logDir ?? "/home/user/.local/share/lightframe/logs");
    if (cmd === "get_log_files") return Promise.resolve(overrides.logFiles ?? sampleLogFiles);
    if (cmd === "set_log_config") return Promise.resolve(undefined);
    if (cmd === "cleanup_logs") return Promise.resolve(undefined);
    return Promise.resolve(null);
  });
}

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  setupInvoke();
});

describe("LogSettings", () => {
  it("renders log settings title and description", async () => {
    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByText("日志设置")).toBeInTheDocument();
    });
    expect(screen.getByText("配置应用日志级别、保留策略和存储限制")).toBeInTheDocument();
  });

  it("loads and displays config from backend", async () => {
    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("14")).toBeInTheDocument();
    });
    expect(screen.getByDisplayValue("200")).toBeInTheDocument();
    expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("debug");
  });

  it("shows log directory and file stats", async () => {
    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByText("/home/user/.local/share/lightframe/logs")).toBeInTheDocument();
    });
    expect(screen.getByText(/日志文件数.*2/)).toBeInTheDocument();
    expect(screen.getByText(/总大小.*1\.5 MB/)).toBeInTheDocument();
  });

  it("saves config when save button clicked", async () => {
    const user = userEvent.setup();
    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "保存" })).toBeInTheDocument();
    });

    await user.selectOptions(screen.getByRole("combobox"), "warn");
    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("set_log_config", {
        config: expect.objectContaining({ level: "warn" }),
      });
      expect(screen.getByRole("button", { name: "已保存 ✓" })).toBeInTheDocument();
    });
  });

  it("cleans up logs and refreshes file list", async () => {
    const user = userEvent.setup();
    let filesCallCount = 0;
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === "get_log_config") return Promise.resolve(defaultConfig);
      if (cmd === "get_log_directory") return Promise.resolve("/logs");
      if (cmd === "get_log_files") {
        filesCallCount += 1;
        return Promise.resolve(filesCallCount > 1 ? [] : sampleLogFiles);
      }
      if (cmd === "cleanup_logs") return Promise.resolve(undefined);
      return Promise.resolve(null);
    });

    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByText(/日志文件数.*2/)).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "立即清理" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("cleanup_logs");
      expect(screen.getByText(/日志文件数.*0/)).toBeInTheDocument();
    });
  });

  it("allows changing retention days", async () => {
    render(<LogSettings />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("14")).toBeInTheDocument();
    });

    const retentionInput = screen.getByDisplayValue("14");
    fireEvent.change(retentionInput, { target: { value: "30" } });

    expect(screen.getByDisplayValue("30")).toBeInTheDocument();
  });
});
