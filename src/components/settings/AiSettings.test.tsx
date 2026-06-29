import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { listen } from "@tauri-apps/api/event";
import { AiSettings } from "./AiSettings";
import { setLocale } from "@/i18n/index";

const getModelStatus = vi.fn();
const openModelsDir = vi.fn();
const downloadModel = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getModelStatus: () => getModelStatus(),
    openModelsDir: () => openModelsDir(),
    downloadModel: (...args: unknown[]) => downloadModel(...args),
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const sampleModels = [
  {
    name: "CLIP Visual",
    filename: "clip-vit-b32-visual.onnx",
    url: "https://example.com/clip.onnx",
    size_mb: 350,
    description: "Similar photo search",
    installed: true,
    file_size_bytes: 350_000_000,
    sha256_verified: true,
  },
  {
    name: "Face Detection",
    filename: "scrfd_500m_bnkps.onnx",
    url: "https://example.com/face.onnx",
    size_mb: 5,
    description: "Face detection",
    installed: false,
    file_size_bytes: null,
    sha256_verified: null,
  },
];

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  vi.clearAllMocks();
  downloadModel.mockResolvedValue("/models/test.onnx");
  delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("AiSettings", () => {
  it("shows model availability status", async () => {
    getModelStatus.mockResolvedValue({
      models_dir: "/home/user/.local/share/lightframe/models",
      clip_available: true,
      face_available: false,
      models: sampleModels,
    });

    render(<AiSettings />);

    await waitFor(() => {
      expect(screen.getAllByText("CLIP Visual").length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText(/已就绪/)).toBeInTheDocument();
      expect(screen.getByText(/未安装/)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "下载" })).toBeInTheDocument();
    });
  });

  it("shows models directory path", async () => {
    getModelStatus.mockResolvedValue({
      models_dir: "/models",
      clip_available: false,
      face_available: false,
      models: sampleModels.map((m) => ({ ...m, installed: false })),
    });

    render(<AiSettings />);

    await waitFor(() => {
      expect(screen.getByText("/models")).toBeInTheDocument();
    });
    expect(screen.getByText("打开模型文件夹")).toBeInTheDocument();
  });

  it("renders full model list with filenames", async () => {
    getModelStatus.mockResolvedValue({
      models_dir: "/models",
      clip_available: true,
      face_available: false,
      models: sampleModels,
    });

    render(<AiSettings />);

    await waitFor(() => {
      expect(screen.getByText(/clip-vit-b32-visual\.onnx/)).toBeInTheDocument();
      expect(screen.getByText(/scrfd_500m_bnkps\.onnx/)).toBeInTheDocument();
      expect(screen.getByText("Similar photo search")).toBeInTheDocument();
      expect(screen.getByText("Face detection")).toBeInTheDocument();
    });
  });

  it("download button calls downloadModel with filename", async () => {
    const user = userEvent.setup();
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

    getModelStatus.mockResolvedValue({
      models_dir: "/models",
      clip_available: false,
      face_available: false,
      models: [
        {
          name: "Face Detection",
          filename: "scrfd_500m_bnkps.onnx",
          url: "https://example.com/face.onnx",
          size_mb: 5,
          description: "Face detection",
          installed: false,
          file_size_bytes: null,
          sha256_verified: null,
        },
      ],
    });

    render(<AiSettings />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "下载" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "下载" }));

    await waitFor(() => {
      expect(downloadModel).toHaveBeenCalledWith("scrfd_500m_bnkps.onnx");
    });
  });

  it("shows download progress bar while downloading", async () => {
    let resolveDownload: (value: string) => void = () => {};
    downloadModel.mockImplementation(
      () =>
        new Promise<string>((resolve) => {
          resolveDownload = resolve;
        }),
    );

    let progressHandler:
      | ((event: { payload: { filename: string; downloaded: number; total: number } }) => void)
      | undefined;
    (listen as ReturnType<typeof vi.fn>).mockImplementation(
      (_event: string, handler: typeof progressHandler) => {
        progressHandler = handler;
        return Promise.resolve(() => {});
      },
    );

    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};

    getModelStatus.mockResolvedValue({
      models_dir: "/models",
      clip_available: false,
      face_available: false,
      models: [
        {
          name: "Face Detection",
          filename: "scrfd_500m_bnkps.onnx",
          url: "https://example.com/face.onnx",
          size_mb: 5,
          description: "Face detection",
          installed: false,
          file_size_bytes: null,
          sha256_verified: null,
        },
      ],
    });

    const user = userEvent.setup();
    render(<AiSettings />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "下载" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("button", { name: "下载" }));

    await waitFor(() => {
      expect(downloadModel).toHaveBeenCalled();
      expect(listen).toHaveBeenCalledWith("model-download-progress", expect.any(Function));
      expect(progressHandler).toBeDefined();
    });

    await act(async () => {
      progressHandler?.({
        payload: {
          filename: "scrfd_500m_bnkps.onnx",
          downloaded: 512_000,
          total: 1_024_000,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/50\.0%/)).toBeInTheDocument();
      expect(screen.getByText(/500 KB/)).toBeInTheDocument();
    });

    resolveDownload("/models/scrfd_500m_bnkps.onnx");
  });
});
