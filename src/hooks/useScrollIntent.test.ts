import { renderHook, act, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useScrollIntent, type ScrollIntent } from "./useScrollIntent";

describe("useScrollIntent", () => {
  let now = 0;
  let container: HTMLDivElement;

  beforeEach(() => {
    now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    container = document.createElement("div");
    Object.defineProperty(container, "scrollTop", {
      writable: true,
      value: 0,
    });
    document.body.appendChild(container);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    document.body.removeChild(container);
  });

  function makeRef(): React.RefObject<HTMLDivElement> {
    return { current: container } as React.RefObject<HTMLDivElement>;
  }

  function scrollTo(top: number, timeMs: number) {
    now = timeMs;
    container.scrollTop = top;
    container.dispatchEvent(new Event("scroll"));
  }

  it("returns idle initially when no scrolling has occurred", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));
    expect(result.current).toBe("idle");
  });

  it("returns slow for scroll speed < 200px/s", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(100, 1000); });

    expect(result.current).toBe("slow");
  });

  it("returns medium for scroll speed 200-1000px/s", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(300, 500); });

    expect(result.current).toBe("medium");
  });

  it("returns fast for scroll speed 1000-3000px/s", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(1000, 500); });

    expect(result.current).toBe("fast");
  });

  it("returns burst for scroll speed > 3000px/s", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(2000, 500); });

    expect(result.current).toBe("burst");
  });

  it("falls back to idle after 300ms of no scrolling", async () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(500, 100); });

    expect(result.current).toBe("burst");

    // Wait for idle timeout (real timer, 300ms)
    await waitFor(() => {
      expect(result.current).toBe("idle");
    }, { timeout: 500 });
  });

  it("handles direction reversal without producing invalid state", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(500, 200); });
    act(() => { scrollTo(200, 300); });

    const valid: ScrollIntent[] = ["idle", "slow", "medium", "fast", "burst"];
    expect(valid).toContain(result.current);
  });

  it("uses absolute velocity for direction changes", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    container.scrollTop = 3000;
    act(() => { scrollTo(3000, 0); });
    act(() => { scrollTo(1000, 500); });

    expect(result.current).toBe("burst");
  });

  it("returns idle when ref is null", () => {
    const ref = { current: null } as React.RefObject<HTMLDivElement | null>;
    const { result } = renderHook(() => useScrollIntent(ref));
    expect(result.current).toBe("idle");
  });

  it("cleans up scroll listener on unmount", () => {
    const ref = makeRef();
    const spy = vi.spyOn(container, "removeEventListener");
    const { unmount } = renderHook(() => useScrollIntent(ref));

    unmount();

    expect(spy).toHaveBeenCalledWith(
      "scroll",
      expect.any(Function),
      expect.anything(),
    );
  });

  it("boundary: exactly 200px/s classifies as medium", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(200, 1000); });

    expect(result.current).toBe("medium");
  });

  it("boundary: exactly 1000px/s classifies as fast", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(1000, 1000); });

    expect(result.current).toBe("fast");
  });

  it("boundary: exactly 3000px/s classifies as burst", () => {
    const ref = makeRef();
    const { result } = renderHook(() => useScrollIntent(ref));

    act(() => { scrollTo(0, 0); });
    act(() => { scrollTo(3000, 1000); });

    expect(result.current).toBe("burst");
  });
});
