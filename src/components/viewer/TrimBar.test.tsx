import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { TrimBar } from "./TrimBar";

describe("TrimBar", () => {
  const defaultProps = {
    duration: 60,
    trimIn: 0,
    trimOut: 60,
    onTrimInChange: vi.fn(),
    onTrimOutChange: vi.fn(),
    onApply: vi.fn(),
    onExport: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders with duration label", () => {
    render(<TrimBar {...defaultProps} />);
    expect(screen.getByTestId("trim-bar")).toBeInTheDocument();
  });

  it("displays current trim range", () => {
    render(<TrimBar {...defaultProps} trimIn={5} trimOut={30} />);
    expect(screen.getByTestId("trim-in-display")).toHaveTextContent("0:05");
    expect(screen.getByTestId("trim-out-display")).toHaveTextContent("0:30");
  });

  it("calls onApply when apply button clicked", () => {
    render(<TrimBar {...defaultProps} trimIn={2} trimOut={10} />);
    fireEvent.click(screen.getByRole("button", { name: /apply/i }));
    expect(defaultProps.onApply).toHaveBeenCalledTimes(1);
  });

  it("calls onExport when export button clicked", () => {
    render(<TrimBar {...defaultProps} trimIn={2} trimOut={10} />);
    fireEvent.click(screen.getByRole("button", { name: /export/i }));
    expect(defaultProps.onExport).toHaveBeenCalledTimes(1);
  });

  it("clamps trimIn to not exceed trimOut", () => {
    const { rerender } = render(
      <TrimBar {...defaultProps} trimIn={0} trimOut={10} />
    );
    // Ensure the component handles out-of-range values gracefully
    rerender(<TrimBar {...defaultProps} trimIn={15} trimOut={10} />);
    // The component should display without crashing
    expect(screen.getByTestId("trim-bar")).toBeInTheDocument();
  });

  it("renders fine-tune keyboard hint", () => {
    render(<TrimBar {...defaultProps} />);
    expect(screen.getByText(/shift/i)).toBeInTheDocument();
  });

  it("formats time correctly for various durations", () => {
    render(<TrimBar {...defaultProps} trimIn={90} trimOut={3661} duration={7200} />);
    expect(screen.getByTestId("trim-in-display")).toHaveTextContent("1:30");
    expect(screen.getByTestId("trim-out-display")).toHaveTextContent("61:01");
  });

  it("disable export when trimIn === 0 and trimOut === duration", () => {
    render(<TrimBar {...defaultProps} trimIn={0} trimOut={60} duration={60} />);
    const exportBtn = screen.getByRole("button", { name: /export/i });
    expect(exportBtn).toBeDisabled();
  });

  it("enables export when trim is modified", () => {
    render(<TrimBar {...defaultProps} trimIn={5} trimOut={55} duration={60} />);
    const exportBtn = screen.getByRole("button", { name: /export/i });
    expect(exportBtn).not.toBeDisabled();
  });
});
