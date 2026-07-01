import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { FaceAnnotationOverlay } from "./FaceAnnotationOverlay";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockInvoke = vi.mocked(invoke);

describe("FaceAnnotationOverlay", () => {
  const defaultProps = {
    mediaId: 1,
    imageWidth: 800,
    imageHeight: 600,
    onClose: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
  });

  it("renders the annotation overlay", () => {
    render(<FaceAnnotationOverlay {...defaultProps} />);
    expect(screen.getByTestId("face-annotation-overlay")).toBeInTheDocument();
  });

  it("creates a bounding box on pointer drag", () => {
    render(<FaceAnnotationOverlay {...defaultProps} />);
    const overlay = screen.getByTestId("face-annotation-overlay");

    fireEvent.pointerDown(overlay, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(overlay, { clientX: 200, clientY: 200 });
    fireEvent.pointerUp(overlay, { clientX: 200, clientY: 200 });

    expect(screen.getByTestId("face-name-input")).toBeInTheDocument();
  });

  it("shows name input with autocomplete after drawing box", async () => {
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "list_person_names") return Promise.resolve(["Alice", "Bob"]);
      return Promise.resolve(null);
    });

    render(<FaceAnnotationOverlay {...defaultProps} />);
    const overlay = screen.getByTestId("face-annotation-overlay");

    fireEvent.pointerDown(overlay, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(overlay, { clientX: 200, clientY: 200 });
    fireEvent.pointerUp(overlay, { clientX: 200, clientY: 200 });

    const input = screen.getByTestId("face-name-input");
    expect(input).toBeInTheDocument();
  });

  it("submits manual face annotation on Enter", async () => {
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "list_person_names") return Promise.resolve([]);
      if (cmd === "create_manual_face") return Promise.resolve(1);
      return Promise.resolve(null);
    });

    render(<FaceAnnotationOverlay {...defaultProps} />);
    const overlay = screen.getByTestId("face-annotation-overlay");

    fireEvent.pointerDown(overlay, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(overlay, { clientX: 300, clientY: 300 });
    fireEvent.pointerUp(overlay, { clientX: 300, clientY: 300 });

    const input = screen.getByTestId("face-name-input");
    fireEvent.change(input, { target: { value: "Alice" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("create_manual_face", expect.objectContaining({
        mediaId: 1,
        personName: "Alice",
      }));
    });
  });

  it("cancels annotation on Escape", () => {
    render(<FaceAnnotationOverlay {...defaultProps} />);
    const overlay = screen.getByTestId("face-annotation-overlay");

    fireEvent.pointerDown(overlay, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(overlay, { clientX: 200, clientY: 200 });
    fireEvent.pointerUp(overlay, { clientX: 200, clientY: 200 });

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("face-name-input")).not.toBeInTheDocument();
  });

  it("does not create box for very small drag (< 10px)", () => {
    render(<FaceAnnotationOverlay {...defaultProps} />);
    expect(screen.getByTestId("face-annotation-overlay")).toBeInTheDocument();
    expect(screen.queryByTestId("face-name-input")).not.toBeInTheDocument();
  });

  it("calls onFaceCreated after successful save", async () => {
    const onFaceCreated = vi.fn();
    mockInvoke.mockImplementation((cmd) => {
      if (cmd === "list_person_names") return Promise.resolve([]);
      if (cmd === "create_manual_face") return Promise.resolve(1);
      return Promise.resolve(null);
    });

    render(<FaceAnnotationOverlay {...defaultProps} onFaceCreated={onFaceCreated} />);
    const overlay = screen.getByTestId("face-annotation-overlay");

    fireEvent.pointerDown(overlay, { clientX: 100, clientY: 100 });
    fireEvent.pointerMove(overlay, { clientX: 300, clientY: 300 });
    fireEvent.pointerUp(overlay, { clientX: 300, clientY: 300 });

    const input = screen.getByTestId("face-name-input");
    fireEvent.change(input, { target: { value: "Alice" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(onFaceCreated).toHaveBeenCalledTimes(1);
    });
  });
});
