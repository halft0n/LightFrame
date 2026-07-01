import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";

export type ScrollIntent = "idle" | "slow" | "medium" | "fast" | "burst";

const IDLE_TIMEOUT_MS = 300;
const SLOW_THRESHOLD = 200; // px/s
const MEDIUM_THRESHOLD = 1000; // px/s
const FAST_THRESHOLD = 3000; // px/s

function classify(velocity: number): ScrollIntent {
  if (velocity < SLOW_THRESHOLD) return "slow";
  if (velocity < MEDIUM_THRESHOLD) return "medium";
  if (velocity < FAST_THRESHOLD) return "fast";
  return "burst";
}

export function useScrollIntent(
  containerRef: React.RefObject<HTMLElement | null>,
): ScrollIntent {
  const intentRef = useRef<ScrollIntent>("idle");
  const lastScrollTop = useRef<number | null>(null);
  const lastTime = useRef<number | null>(null);
  const idleTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const subscribersRef = useRef<Set<() => void>>(new Set());

  const subscribe = useCallback((cb: () => void) => {
    subscribersRef.current.add(cb);
    return () => { subscribersRef.current.delete(cb); };
  }, []);

  const getSnapshot = useCallback(() => intentRef.current, []);

  const notify = useCallback(() => {
    for (const cb of subscribersRef.current) cb();
  }, []);

  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;

    const now = performance.now();
    const scrollTop = el.scrollTop;

    if (lastScrollTop.current !== null && lastTime.current !== null) {
      const dt = now - lastTime.current;
      if (dt > 0) {
        const distance = Math.abs(scrollTop - lastScrollTop.current);
        const velocity = (distance / dt) * 1000;
        const newIntent = classify(velocity);
        if (newIntent !== intentRef.current) {
          intentRef.current = newIntent;
          notify();
        }
      }
    }

    lastScrollTop.current = scrollTop;
    lastTime.current = now;

    if (idleTimer.current !== null) {
      clearTimeout(idleTimer.current);
    }
    idleTimer.current = setTimeout(() => {
      if (intentRef.current !== "idle") {
        intentRef.current = "idle";
        notify();
      }
      lastScrollTop.current = null;
      lastTime.current = null;
    }, IDLE_TIMEOUT_MS);
  }, [containerRef, notify]);

  useEffect(() => {
    let attachedEl: HTMLElement | null = null;
    const opts: AddEventListenerOptions = { passive: true };

    const attach = (el: HTMLElement) => {
      attachedEl = el;
      el.addEventListener("scroll", handleScroll, opts);
    };

    const el = containerRef.current;
    if (el) {
      attach(el);
    } else {
      // Container not yet mounted — observe DOM until it appears
      const observer = new MutationObserver(() => {
        const target = containerRef.current;
        if (target) {
          observer.disconnect();
          attach(target);
        }
      });
      observer.observe(document.body, { childList: true, subtree: true });

      return () => {
        observer.disconnect();
        if (attachedEl) {
          attachedEl.removeEventListener("scroll", handleScroll, opts);
        }
        if (idleTimer.current !== null) {
          clearTimeout(idleTimer.current);
        }
      };
    }

    return () => {
      if (attachedEl) {
        attachedEl.removeEventListener("scroll", handleScroll, opts);
      }
      if (idleTimer.current !== null) {
        clearTimeout(idleTimer.current);
      }
    };
  }, [containerRef, handleScroll]);

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
