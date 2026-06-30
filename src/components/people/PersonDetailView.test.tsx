import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PersonDetailView } from "./PersonDetailView";
import { setLocale } from "@/i18n/index";
import { openPersonDetail, closePersonDetail } from "@/store/appStore";

const listPersons = vi.fn();
const getPersonFaces = vi.fn();
const renamePerson = vi.fn();
const splitFaceFromPerson = vi.fn();

vi.mock("@/lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/tauri")>();
  return {
    ...actual,
    listPersons: (...args: unknown[]) => listPersons(...args),
    getPersonFaces: (...args: unknown[]) => getPersonFaces(...args),
    renamePerson: (...args: unknown[]) => renamePerson(...args),
    splitFaceFromPerson: (...args: unknown[]) => splitFaceFromPerson(...args),
    getFaceThumbnailUrl: (faceId: number) => `face://localhost/${faceId}`,
  };
});

const samplePerson = {
  id: 42,
  name: "Carol",
  face_count: 2,
  cover_face_id: 100,
  sample_media_ids: [500],
  created_at: "2024-03-01",
};

const sampleFaces = [
  {
    id: 100,
    media_id: 500,
    confidence: 0.98,
    bbox: [0, 0, 50, 50] as [number, number, number, number],
    person_id: 42,
  },
  {
    id: 101,
    media_id: 501,
    confidence: 0.95,
    bbox: [10, 10, 60, 60] as [number, number, number, number],
    person_id: 42,
  },
];

beforeEach(() => {
  localStorage.clear();
  setLocale("zh-CN");
  closePersonDetail();
  listPersons.mockReset();
  getPersonFaces.mockReset();
  renamePerson.mockReset();
  splitFaceFromPerson.mockReset();
  listPersons.mockResolvedValue([samplePerson]);
  getPersonFaces.mockResolvedValue(sampleFaces);
  splitFaceFromPerson.mockResolvedValue(undefined);
});

describe("PersonDetailView", () => {
  it("renders person detail with face count", async () => {
    openPersonDetail(42);
    render(<PersonDetailView />);

    await waitFor(() => {
      expect(screen.getByText("Carol")).toBeInTheDocument();
      expect(screen.getByText("2 张照片")).toBeInTheDocument();
    });
  });

  it("renders face thumbnails", async () => {
    openPersonDetail(42);
    const { container } = render(<PersonDetailView />);

    await waitFor(() => {
      expect(container.querySelectorAll("img").length).toBe(2);
    });

    const images = container.querySelectorAll("img");
    expect(images[0]).toHaveAttribute("src", "face://localhost/100");
    expect(images[1]).toHaveAttribute("src", "face://localhost/101");
  });

  it("remove from person button splits face", async () => {
    const user = userEvent.setup();
    openPersonDetail(42);
    render(<PersonDetailView />);

    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: "从人物中移除" }).length,
      ).toBe(2);
    });

    await user.click(
      screen.getAllByRole("button", { name: "从人物中移除" })[0],
    );

    await waitFor(() => {
      expect(splitFaceFromPerson).toHaveBeenCalledWith(100);
    });
  });

  it("returns null when no person is selected", () => {
    const { container } = render(<PersonDetailView />);
    expect(container.firstChild).toBeNull();
  });
});
