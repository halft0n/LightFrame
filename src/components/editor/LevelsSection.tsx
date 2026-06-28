import { useCallback, useMemo } from "react";
import { DEFAULT_LEVELS, type EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { EditorSection } from "./EditorSection";

interface LevelsSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

function clamp(v: number, min: number, max: number) {
  return Math.min(max, Math.max(min, v));
}

export function LevelsSection({ params, onChange }: LevelsSectionProps) {
  const { t } = useTranslation();
  const levels = params.levels ?? { ...DEFAULT_LEVELS };

  const updateLevels = useCallback(
    (patch: Partial<typeof DEFAULT_LEVELS>) => {
      onChange({ levels: { ...levels, ...patch } });
    },
    [levels, onChange],
  );

  const histogramBars = useMemo(() => {
    const bars: number[] = [];
    let seed = 42;
    for (let i = 0; i < 64; i++) {
      seed = (seed * 1664525 + 1013904223) & 0x7fffffff;
      const pseudo = (seed % 1000) / 10000;
      bars.push(0.2 + Math.sin(i * 0.3) * 0.15 + pseudo);
    }
    return bars;
  }, []);

  const inputBlack = levels.inputBlack;
  const inputWhite = levels.inputWhite;
  const gamma = levels.gamma;
  const outputBlack = levels.outputBlack;
  const outputWhite = levels.outputWhite;

  const renderHandleRow = (
    label: string,
    value: number,
    min: number,
    max: number,
    step: number,
    onUpdate: (v: number) => void,
  ) => (
    <div className="py-1.5">
      <div className="mb-1 flex items-center justify-between text-xs">
        <span className="text-neutral-400">{label}</span>
        <input
          type="number"
          min={min}
          max={max}
          step={step}
          value={step < 1 ? value.toFixed(1) : value}
          onChange={(e) => onUpdate(clamp(Number(e.target.value), min, max))}
          className="w-14 rounded bg-white/10 px-1.5 py-0.5 text-right tabular-nums text-neutral-300 outline-none focus:ring-1 focus:ring-blue-500"
        />
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onUpdate(Number(e.target.value))}
        className="editor-slider w-full cursor-pointer appearance-none bg-transparent"
        aria-label={label}
      />
    </div>
  );

  return (
    <EditorSection title={t("editor.levels")} defaultOpen={false}>
      <div className="relative mb-2 h-16 overflow-hidden rounded-lg bg-[#1a1a1a]">
        <div className="absolute inset-0 flex items-end gap-px px-1 pb-1">
          {histogramBars.map((h, i) => (
            <div
              key={i}
              className="flex-1 rounded-t bg-neutral-600/60"
              style={{ height: `${h * 100}%` }}
            />
          ))}
        </div>
        <div
          className="pointer-events-none absolute bottom-0 top-0 border-l-2 border-black"
          style={{ left: `${(inputBlack / 255) * 100}%` }}
        />
        <div
          className="pointer-events-none absolute bottom-0 top-0 border-r-2 border-white"
          style={{ right: `${((255 - inputWhite) / 255) * 100}%` }}
        />
      </div>

      <p className="text-xs font-medium text-neutral-500">{t("editor.levels.inputBlack")}</p>
      {renderHandleRow(t("editor.levels.inputBlack"), inputBlack, 0, 254, 1, (v) =>
        updateLevels({ inputBlack: Math.min(v, inputWhite - 1) }),
      )}
      {renderHandleRow(t("editor.levels.gamma"), gamma, 0.1, 9.9, 0.1, (v) =>
        updateLevels({ gamma: v }),
      )}
      {renderHandleRow(t("editor.levels.inputWhite"), inputWhite, 1, 255, 1, (v) =>
        updateLevels({ inputWhite: Math.max(v, inputBlack + 1) }),
      )}

      <p className="pt-2 text-xs font-medium text-neutral-500">{t("editor.levels.outputBlack")}</p>
      {renderHandleRow(t("editor.levels.outputBlack"), outputBlack, 0, 254, 1, (v) =>
        updateLevels({ outputBlack: Math.min(v, outputWhite - 1) }),
      )}
      {renderHandleRow(t("editor.levels.outputWhite"), outputWhite, 1, 255, 1, (v) =>
        updateLevels({ outputWhite: Math.max(v, outputBlack + 1) }),
      )}
    </EditorSection>
  );
}
