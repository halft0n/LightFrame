import { useCallback, useRef, useEffect } from "react";

export interface LongPressOptions {
  delay?: number;
  moveThreshold?: number;
}

export interface LongPressHandlers {
  onPointerDown: (e: React.PointerEvent) => void;
  onPointerUp: (e: React.PointerEvent) => void;
  onPointerMove: (e: React.PointerEvent) => void;
  onPointerCancel: (e: React.PointerEvent) => void;
}

export function useLongPress(
  onLongPress: () => void,
  options?: LongPressOptions
): LongPressHandlers {
  const { delay = 130, moveThreshold = 5 } = options ?? {};

  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const startPos = useRef<{ x: number; y: number } | null>(null);
  const firedRef = useRef(false);
  const callbackRef = useRef(onLongPress);
  callbackRef.current = onLongPress;

  const cancel = useCallback(() => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    startPos.current = null;
  }, []);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      startPos.current = { x: e.clientX, y: e.clientY };
      firedRef.current = false;
      timerRef.current = setTimeout(() => {
        if (!firedRef.current) {
          firedRef.current = true;
          callbackRef.current();
        }
        timerRef.current = null;
      }, delay);
    },
    [delay]
  );

  const onPointerUp = useCallback(
    (_e: React.PointerEvent) => {
      cancel();
    },
    [cancel]
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!startPos.current) return;
      const dx = Math.abs(e.clientX - startPos.current.x);
      const dy = Math.abs(e.clientY - startPos.current.y);
      if (dx > moveThreshold || dy > moveThreshold) {
        cancel();
      }
    },
    [moveThreshold, cancel]
  );

  const onPointerCancel = useCallback(
    (_e: React.PointerEvent) => {
      cancel();
    },
    [cancel]
  );

  return { onPointerDown, onPointerUp, onPointerMove, onPointerCancel };
}
