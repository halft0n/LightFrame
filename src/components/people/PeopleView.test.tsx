import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PeopleView } from "./PeopleView";
import { setLocale } from "@/i18n/index";

const listPersons = vi.fn();
const getAiStatus = vi.fn();
const clusterFaces = vi.fn();
const renamePerson = vi.fn();
const detectFacesBatch = vi.fn();
const mergePersons = vi.fn();
const onFaceDetectionProgress = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listPersons: (...args: unknown[]) => listPersons(...args),
    getAiStatus: (...args: unknown[]) => getAiStatus(...args),
    clusterFaces: (...args: unknown[]) => clusterFaces(...args),
    renamePerson: (...args: unknown[]) => renamePerson(...args),
    detectFacesBatch: (...args: unknown[]) => detectFacesBatch(...args),
    mergePersons: (...args: unknown[]) => mergePersons(...args),
    onFaceDetectionProgress: (...args: unknown[]) =>
      onFaceDetectionProgress(...args),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  listPersons.mockReset();
  getAiStatus.mockReset();
  clusterFaces.mockReset();
  renamePerson.mockReset();
  detectFacesBatch.mockReset();
  mergePersons.mockReset();
  onFaceDetectionProgress.mockResolvedValue(() => {});
  detectFacesBatch.mockResolvedValue({ faces_found: 3 });
  mergePersons.mockResolvedValue(undefined);
});

describe("PeopleView", () => {
  it("shows AI required message when no AI", async () => {
    listPersons.mockResolvedValue([]);
    getAiStatus.mockResolvedValue({
      python_available: false,
      face_available: false,
      clip_available: false,
      status_message: "Python sidecar not running",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(screen.getByText("暂无识别的人物")).toBeInTheDocument();
    });
    expect(
      screen.getByText("AI 人脸识别功能需要安装 Python 扩展"),
    ).toBeInTheDocument();
  });

  it("shows person list with face counts", async () => {
    listPersons.mockResolvedValue([
      {
        id: 1,
        name: "Alice",
        face_count: 5,
        cover_face_id: null,
        sample_media_ids: [10],
        created_at: "2024-01-01",
      },
    ]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(screen.getByText("Alice")).toBeInTheDocument();
      expect(screen.getByText("5 张照片")).toBeInTheDocument();
    });
  });

  it("cluster button triggers clustering", async () => {
    const user = userEvent.setup();
    listPersons.mockResolvedValue([]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });
    clusterFaces.mockResolvedValue(undefined);

    render(<PeopleView />);
    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: "聚类人脸" }).length,
      ).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByRole("button", { name: "聚类人脸" })[0]);
    await waitFor(() => {
      expect(clusterFaces).toHaveBeenCalled();
    });
  });

  it("rename person inline editing", async () => {
    const user = userEvent.setup();
    listPersons.mockResolvedValue([
      {
        id: 2,
        name: "Bob",
        face_count: 1,
        cover_face_id: null,
        sample_media_ids: [],
        created_at: "2024-01-01",
      },
    ]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });
    renamePerson.mockResolvedValue(undefined);

    render(<PeopleView />);
    await waitFor(() => {
      expect(screen.getByText("Bob")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Bob"));
    const input = screen.getByDisplayValue("Bob");
    await user.clear(input);
    await user.type(input, "Robert");
    await user.tab();

    await waitFor(() => {
      expect(renamePerson).toHaveBeenCalledWith(2, "Robert");
    });
  });

  it("shows empty state", async () => {
    listPersons.mockResolvedValue([]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(screen.getByText("暂无识别的人物")).toBeInTheDocument();
    });
  });

  it("renders detect faces button when AI is ready", async () => {
    listPersons.mockResolvedValue([]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: "检测人脸" }).length,
      ).toBeGreaterThan(0);
    });
  });

  it("detect faces button triggers batch detection", async () => {
    const user = userEvent.setup();
    listPersons.mockResolvedValue([]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: "检测人脸" }).length,
      ).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByRole("button", { name: "检测人脸" })[0]);

    await waitFor(() => {
      expect(detectFacesBatch).toHaveBeenCalled();
      expect(clusterFaces).toHaveBeenCalled();
    });
  });

  it("merge selection UI appears when two people are selected", async () => {
    const user = userEvent.setup();
    listPersons.mockResolvedValue([
      {
        id: 1,
        name: "Alice",
        face_count: 2,
        cover_face_id: null,
        sample_media_ids: [10],
        created_at: "2024-01-01",
      },
      {
        id: 2,
        name: "Bob",
        face_count: 3,
        cover_face_id: null,
        sample_media_ids: [11],
        created_at: "2024-01-02",
      },
    ]);
    getAiStatus.mockResolvedValue({
      python_available: true,
      face_available: true,
      clip_available: true,
      status_message: "ready",
    });

    render(<PeopleView />);
    await waitFor(() => {
      expect(screen.getByText("Alice")).toBeInTheDocument();
      expect(screen.getByText("Bob")).toBeInTheDocument();
    });

    const selectButtons = screen.getAllByRole("button", { name: "选择人物" });
    await user.click(selectButtons[0]);
    await user.click(selectButtons[1]);

    const mergeButton = await screen.findByRole("button", {
      name: "合并 2 人",
    });
    await user.click(mergeButton);

    await waitFor(() => {
      expect(mergePersons).toHaveBeenCalledWith([1, 2]);
    });
  });
});
