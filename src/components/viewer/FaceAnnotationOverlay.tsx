import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FaceAnnotationOverlayProps {
  mediaId: number;
  imageWidth: number;
  imageHeight: number;
  onClose: () => void;
  onFaceCreated?: () => void;
}

interface BBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

const MIN_BOX_SIZE = 10;

export function FaceAnnotationOverlay({
  mediaId,
  imageWidth,
  imageHeight,
  onClose,
  onFaceCreated,
}: FaceAnnotationOverlayProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const [drawing, setDrawing] = useState(false);
  const startPosRef = useRef<{ x: number; y: number } | null>(null);
  const currentPosRef = useRef<{ x: number; y: number } | null>(null);
  const [currentPos, setCurrentPos] = useState<{ x: number; y: number } | null>(null);
  const [drawnBox, setDrawnBox] = useState<BBox | null>(null);
  const drawnBoxRef = useRef<BBox | null>(null);
  const [nameValue, setNameValue] = useState("");
  const [personNames, setPersonNames] = useState<string[]>([]);

  useEffect(() => {
    invoke<string[]>("list_person_names")
      .then(setPersonNames)
      .catch((err) => console.warn("Failed to load person names:", err));
  }, []);

  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (drawnBox) {
          setDrawnBox(null);
          drawnBoxRef.current = null;
          setNameValue("");
        } else {
          onClose();
        }
      }
    };
    document.addEventListener("keydown", handleEsc);
    return () => document.removeEventListener("keydown", handleEsc);
  }, [drawnBox, onClose]);

  const getRelativePos = useCallback(
    (clientX: number, clientY: number) => {
      const rect = overlayRef.current?.getBoundingClientRect();
      if (!rect || rect.width === 0 || rect.height === 0) {
        return { x: clientX, y: clientY };
      }
      // TODO: Coordinates are mapped to the overlay rect, not the letterboxed
      // image rect inside it. Correct mapping requires runtime DOM measurement of
      // the rendered image vs overlay (not feasible in jsdom).
      return {
        x: Math.max(0, Math.min(clientX - rect.left, rect.width)),
        y: Math.max(0, Math.min(clientY - rect.top, rect.height)),
      };
    },
    []
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (drawnBoxRef.current) return;
      const pos = getRelativePos(e.clientX, e.clientY);
      startPosRef.current = pos;
      currentPosRef.current = pos;
      setCurrentPos(pos);
      setDrawing(true);
      try {
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      } catch {
        // jsdom doesn't support setPointerCapture
      }
    },
    [getRelativePos]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!startPosRef.current) return;
      const pos = getRelativePos(e.clientX, e.clientY);
      currentPosRef.current = pos;
      setCurrentPos(pos);
    },
    [getRelativePos]
  );

  const handlePointerUp = useCallback(
    (_e: React.PointerEvent) => {
      const startPos = startPosRef.current;
      const endPos = currentPosRef.current;
      if (!startPos || !endPos) {
        setDrawing(false);
        return;
      }
      setDrawing(false);

      const dx = Math.abs(endPos.x - startPos.x);
      const dy = Math.abs(endPos.y - startPos.y);

      if (dx < MIN_BOX_SIZE || dy < MIN_BOX_SIZE) {
        startPosRef.current = null;
        currentPosRef.current = null;
        setCurrentPos(null);
        return;
      }

      const rect = overlayRef.current?.getBoundingClientRect();
      const rw = rect && rect.width > 0 ? rect.width : imageWidth;
      const rh = rect && rect.height > 0 ? rect.height : imageHeight;

      const x = Math.min(startPos.x, endPos.x) / rw;
      const y = Math.min(startPos.y, endPos.y) / rh;
      const w = dx / rw;
      const h = dy / rh;

      setDrawnBox({ x, y, w, h });
      drawnBoxRef.current = { x, y, w, h };
      startPosRef.current = null;
      currentPosRef.current = null;
    },
    [imageWidth, imageHeight]
  );

  const handleSubmit = useCallback(async () => {
    if (!drawnBox || !nameValue.trim()) return;
    const bbox: [number, number, number, number] = [
      drawnBox.x,
      drawnBox.y,
      drawnBox.x + drawnBox.w,
      drawnBox.y + drawnBox.h,
    ];
    try {
      await invoke("create_manual_face", {
        mediaId,
        bbox,
        personName: nameValue.trim(),
      });
      onFaceCreated?.();
    } catch (err) {
      console.warn("Failed to create manual face:", err);
    }
    setDrawnBox(null);
    drawnBoxRef.current = null;
    setNameValue("");
  }, [drawnBox, nameValue, mediaId, onFaceCreated]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit]
  );

  const drawingBoxStyle = (() => {
    if (!drawing || !startPosRef.current || !currentPos) return undefined;
    const sp = startPosRef.current;
    const left = Math.min(sp.x, currentPos.x);
    const top = Math.min(sp.y, currentPos.y);
    const width = Math.abs(currentPos.x - sp.x);
    const height = Math.abs(currentPos.y - sp.y);
    return { left, top, width, height };
  })();

  return (
    <div
      ref={overlayRef}
      data-testid="face-annotation-overlay"
      className="absolute inset-0 cursor-crosshair"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      {drawingBoxStyle && (
        <div
          className="absolute border-2 border-blue-400 bg-blue-400/20"
          style={drawingBoxStyle}
        />
      )}

      {drawnBox && (
        <div
          className="absolute border-2 border-green-400 bg-green-400/20"
          style={{
            left: `${drawnBox.x * 100}%`,
            top: `${drawnBox.y * 100}%`,
            width: `${drawnBox.w * 100}%`,
            height: `${drawnBox.h * 100}%`,
          }}
        >
          <input
            data-testid="face-name-input"
            type="text"
            value={nameValue}
            onChange={(e) => setNameValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Name..."
            autoFocus
            list="person-name-suggestions"
            className="absolute -bottom-8 left-0 w-32 rounded bg-black/80 px-2 py-1 text-xs text-white outline-none focus:ring-1 focus:ring-blue-400"
          />
          <datalist id="person-name-suggestions">
            {personNames.map((name) => (
              <option key={name} value={name} />
            ))}
          </datalist>
        </div>
      )}
    </div>
  );
}
