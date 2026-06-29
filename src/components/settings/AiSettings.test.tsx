import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { AiSettings } from "./AiSettings";
import { setLocale } from "@/i18n/index";

const getModelStatus = vi.fn();
const openModelsDir = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    getModelStatus: () => getModelStatus(),
    openModelsDir: () => openModelsDir(),
    downloadModel: vi.fn(),
  };
});

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
});
