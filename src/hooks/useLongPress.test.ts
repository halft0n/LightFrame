import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useLongPress } from "./useLongPress";

describe("useLongPress", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("fires callback after 130ms hold without movement", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    expect(onLongPress).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(130);
    });

    expect(onLongPress).toHaveBeenCalledTimes(1);
  });

  it("does not fire if pointer released before 130ms", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(80);
    });

    act(() => {
      result.current.onPointerUp({} as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("cancels if pointer moves more than 5px horizontally", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      result.current.onPointerMove({
        clientX: 106,
        clientY: 100,
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("cancels if pointer moves more than 5px vertically", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      result.current.onPointerMove({
        clientX: 100,
        clientY: 106,
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("does not cancel for small movement within 5px threshold", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      result.current.onPointerMove({
        clientX: 103,
        clientY: 104,
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(130);
    });

    expect(onLongPress).toHaveBeenCalledTimes(1);
  });

  it("cancels on pointerCancel event", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      result.current.onPointerCancel({} as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("cleans up timer on unmount", () => {
    const onLongPress = vi.fn();
    const { result, unmount } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    unmount();

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("does not fire multiple times for sustained hold", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(onLongPress).toHaveBeenCalledTimes(1);
  });

  it("allows configurable delay", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress, { delay: 300 }));

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(130);
    });
    expect(onLongPress).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(170);
    });
    expect(onLongPress).toHaveBeenCalledTimes(1);
  });

  it("allows configurable move threshold", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() =>
      useLongPress(onLongPress, { moveThreshold: 20 })
    );

    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });

    // Move 15px - should NOT cancel with threshold 20
    act(() => {
      result.current.onPointerMove({
        clientX: 115,
        clientY: 100,
      } as unknown as React.PointerEvent);
    });

    act(() => {
      vi.advanceTimersByTime(130);
    });

    expect(onLongPress).toHaveBeenCalledTimes(1);
  });

  it("can be triggered again after releasing", () => {
    const onLongPress = vi.fn();
    const { result } = renderHook(() => useLongPress(onLongPress));

    // First press
    act(() => {
      result.current.onPointerDown({
        clientX: 100,
        clientY: 100,
        pointerId: 1,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });
    act(() => { vi.advanceTimersByTime(130); });
    act(() => { result.current.onPointerUp({} as React.PointerEvent); });

    expect(onLongPress).toHaveBeenCalledTimes(1);

    // Second press
    act(() => {
      result.current.onPointerDown({
        clientX: 200,
        clientY: 200,
        pointerId: 2,
        preventDefault: vi.fn(),
      } as unknown as React.PointerEvent);
    });
    act(() => { vi.advanceTimersByTime(130); });

    expect(onLongPress).toHaveBeenCalledTimes(2);
  });
});
