import { createPortal } from "react-dom";
import type { ReactNode } from "react";

interface ViewerOverlayProps {
  children: ReactNode;
}

export function ViewerOverlay({ children }: ViewerOverlayProps) {
  return createPortal(
    <div className="fixed inset-0 z-50">{children}</div>,
    document.body,
  );
}
