interface TrimBarProps {
  duration: number;
  trimIn: number;
  trimOut: number;
  onTrimInChange: (value: number) => void;
  onTrimOutChange: (value: number) => void;
  onApply: () => void;
  onExport: () => void;
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function TrimBar({
  duration,
  trimIn,
  trimOut,
  onTrimInChange,
  onTrimOutChange,
  onApply,
  onExport,
}: TrimBarProps) {
  const isUnmodified = trimIn === 0 && trimOut === duration;
  const safeIn = Math.min(trimIn, trimOut);
  const safeOut = Math.max(trimIn, trimOut);

  const handleInChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Math.min(parseFloat(e.target.value), safeOut - 0.1);
    onTrimInChange(Math.max(0, val));
  };

  const handleOutChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = Math.max(parseFloat(e.target.value), safeIn + 0.1);
    onTrimOutChange(Math.min(duration, val));
  };

  const handleKeyDown = (e: React.KeyboardEvent, which: "in" | "out") => {
    if (!e.shiftKey) return;
    const step = 0.1;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (which === "in") onTrimInChange(Math.max(0, safeIn - step));
      else onTrimOutChange(Math.max(safeIn + 0.1, safeOut - step));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      if (which === "in") onTrimInChange(Math.min(safeOut - 0.1, safeIn + step));
      else onTrimOutChange(Math.min(duration, safeOut + step));
    }
  };

  const highlightLeft = duration > 0 ? (safeIn / duration) * 100 : 0;
  const highlightWidth = duration > 0 ? ((safeOut - safeIn) / duration) * 100 : 100;

  return (
    <div data-testid="trim-bar" className="flex flex-col gap-2 rounded-lg bg-neutral-900/80 p-3">
      <div className="relative h-6 w-full rounded bg-neutral-700">
        <div
          className="absolute top-0 h-full rounded bg-blue-500/40"
          style={{ left: `${highlightLeft}%`, width: `${highlightWidth}%` }}
        />
        <input
          type="range"
          min={0}
          max={duration}
          step={0.1}
          value={safeIn}
          onChange={handleInChange}
          onKeyDown={(e) => handleKeyDown(e, "in")}
          className="trim-range trim-range-in absolute inset-0 h-full w-full cursor-pointer appearance-none bg-transparent opacity-60"
          aria-label="Trim start"
        />
        <input
          type="range"
          min={0}
          max={duration}
          step={0.1}
          value={safeOut}
          onChange={handleOutChange}
          onKeyDown={(e) => handleKeyDown(e, "out")}
          className="trim-range trim-range-out absolute inset-0 h-full w-full cursor-pointer appearance-none bg-transparent"
          aria-label="Trim end"
        />
      </div>

      <div className="flex items-center justify-between text-xs text-neutral-300">
        <span data-testid="trim-in-display">{formatTime(safeIn)}</span>
        <span className="text-neutral-500">Shift + ←/→ fine-tune 0.1s</span>
        <span data-testid="trim-out-display">{formatTime(safeOut)}</span>
      </div>

      <div className="flex gap-2">
        <button
          type="button"
          onClick={onApply}
          className="flex-1 rounded bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700"
          aria-label="Apply"
        >
          Apply
        </button>
        <button
          type="button"
          onClick={onExport}
          disabled={isUnmodified}
          className="flex-1 rounded bg-green-600 px-3 py-1 text-xs font-medium text-white hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
          aria-label="Export"
        >
          Export
        </button>
      </div>
    </div>
  );
}
