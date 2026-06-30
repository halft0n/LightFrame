import { useCallback } from "react";
import { formatSliderValue } from "@/lib/editParams";

interface AdjustmentSliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  defaultValue?: number;
  onChange: (value: number) => void;
}

export function AdjustmentSlider({
  label,
  value,
  min,
  max,
  defaultValue = 0,
  onChange,
}: AdjustmentSliderProps) {
  const hasCenter = min < 0 && max > 0;
  const centerPercent = hasCenter ? ((0 - min) / (max - min)) * 100 : null;

  const handleDoubleClick = useCallback(() => {
    onChange(defaultValue);
  }, [defaultValue, onChange]);

  return (
    <div className="group py-2">
      <div className="mb-1.5 flex items-center justify-between text-xs">
        <span className="text-neutral-400">{label}</span>
        <span className="tabular-nums text-neutral-300">
          {formatSliderValue(value)}
        </span>
      </div>
      <div className="relative">
        {centerPercent != null && (
          <div
            className="pointer-events-none absolute top-1/2 z-0 h-2 w-px -translate-y-1/2 bg-neutral-600"
            style={{ left: `${centerPercent}%` }}
          />
        )}
        <input
          type="range"
          min={min}
          max={max}
          step={1}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          onDoubleClick={handleDoubleClick}
          className="editor-slider relative z-10 w-full cursor-pointer appearance-none bg-transparent"
          aria-label={label}
        />
      </div>
    </div>
  );
}
