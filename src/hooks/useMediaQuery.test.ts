import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useMediaQuery, MOBILE_BREAKPOINT, useIsMobile } from "./useMediaQuery";

type MediaQueryListener = (event: MediaQueryListEvent) => void;

function createMatchMediaMock(initialMatches: boolean) {
  let matches = initialMatches;
  const listeners = new Set<MediaQueryListener>();

  const mql = {
    get matches() {
      return matches;
    },
    media: "",
    addEventListener(_type: string, handler: MediaQueryListener) {
      listeners.add(handler);
    },
    removeEventListener(_type: string, handler: MediaQueryListener) {
      listeners.delete(handler);
    },
    dispatch(next: boolean) {
      matches = next;
      const event = { matches: next } as MediaQueryListEvent;
      for (const listener of listeners) {
        listener(event);
      }
    },
  } as MediaQueryList & { dispatch: (next: boolean) => void };

  const matchMedia = vi.fn((_query: string) => mql);

  return { matchMedia, mql };
}

describe("useMediaQuery", () => {
  let originalMatchMedia: typeof window.matchMedia;

  beforeEach(() => {
    originalMatchMedia = window.matchMedia;
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it("returns true when query initially matches", () => {
    const { matchMedia } = createMatchMediaMock(true);
    window.matchMedia = matchMedia;

    const { result } = renderHook(() => useMediaQuery("(max-width: 767px)"));
    expect(result.current).toBe(true);
    expect(matchMedia).toHaveBeenCalledWith("(max-width: 767px)");
  });

  it("returns false when query does not match", () => {
    const { matchMedia } = createMatchMediaMock(false);
    window.matchMedia = matchMedia;

    const { result } = renderHook(() => useMediaQuery("(min-width: 1024px)"));
    expect(result.current).toBe(false);
  });

  it("updates when media query changes", () => {
    const { matchMedia, mql } = createMatchMediaMock(false);
    window.matchMedia = matchMedia;

    const { result } = renderHook(() => useMediaQuery("(max-width: 767px)"));
    expect(result.current).toBe(false);

    act(() => {
      mql.dispatch(true);
    });
    expect(result.current).toBe(true);

    act(() => {
      mql.dispatch(false);
    });
    expect(result.current).toBe(false);
  });

  it("removes listener on unmount", () => {
    const { matchMedia, mql } = createMatchMediaMock(true);
    window.matchMedia = matchMedia;

    const { result, unmount } = renderHook(() => useMediaQuery("(max-width: 767px)"));
    expect(result.current).toBe(true);

    unmount();

    act(() => {
      mql.dispatch(false);
    });
    expect(result.current).toBe(true);
  });

  it("re-subscribes when query string changes", () => {
    const desktop = createMatchMediaMock(false);
    const mobile = createMatchMediaMock(true);
    window.matchMedia = vi.fn((query: string) =>
      query.includes("767px") ? mobile.mql : desktop.mql,
    );

    const { result, rerender } = renderHook(({ query }) => useMediaQuery(query), {
      initialProps: { query: "(min-width: 768px)" },
    });
    expect(result.current).toBe(false);

    rerender({ query: "(max-width: 767px)" });
    expect(result.current).toBe(true);
    expect(window.matchMedia).toHaveBeenCalledWith("(max-width: 767px)");
  });
});

describe("useIsMobile", () => {
  it("uses the mobile breakpoint constant", () => {
    const { matchMedia } = createMatchMediaMock(true);
    window.matchMedia = matchMedia;

    const { result } = renderHook(() => useIsMobile());
    expect(result.current).toBe(true);
    expect(matchMedia).toHaveBeenCalledWith(MOBILE_BREAKPOINT);
  });
});
