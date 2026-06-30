import { useCallback, useState } from "react";
import type { EditParams, SelectiveColorChannel } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

type ColorKey = "reds" | "yellows" | "greens" | "cyans" | "blues" | "magentas";

const COLOR_TABS: { key: ColorKey; labelKey: string; dot: string }[] = [
  { key: "reds", labelKey: "editor.selectiveColor.reds", dot: "bg-red-500" },
  {
    key: "yellows",
    labelKey: "editor.selectiveColor.yellows",
    dot: "bg-yellow-400",
  },
  {
    key: "greens",
    labelKey: "editor.selectiveColor.greens",
    dot: "bg-green-500",
  },
  { key: "cyans", labelKey: "editor.selectiveColor.cyans", dot: "bg-cyan-400" },
  { key: "blues", labelKey: "editor.selectiveColor.blues", dot: "bg-blue-500" },
  {
    key: "magentas",
    labelKey: "editor.selectiveColor.magentas",
    dot: "bg-fuchsia-500",
  },
];

const DEFAULT_CHANNEL: SelectiveColorChannel = {
  hue: 0,
  saturation: 0,
  luminance: 0,
};

interface SelectiveColorSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

export function SelectiveColorSection({
  params,
  onChange,
}: SelectiveColorSectionProps) {
  const { t } = useTranslation();
  const [active, setActive] = useState<ColorKey>("reds");
  const selectiveColor = params.selectiveColor ?? {};

  const current = selectiveColor[active] ?? { ...DEFAULT_CHANNEL };

  const updateChannel = useCallback(
    (patch: Partial<SelectiveColorChannel>) => {
      onChange({
        selectiveColor: {
          ...selectiveColor,
          [active]: { ...current, ...patch },
        },
      });
    },
    [active, current, onChange, selectiveColor],
  );

  const resetTab = () => {
    onChange({
      selectiveColor: {
        ...selectiveColor,
        [active]: { ...DEFAULT_CHANNEL },
      },
    });
  };

  const isDefault =
    current.hue === 0 && current.saturation === 0 && current.luminance === 0;

  return (
    <EditorSection title={t("editor.selectiveColor")} defaultOpen={false}>
      <div className="grid grid-cols-3 gap-1.5">
        {COLOR_TABS.map(({ key, labelKey, dot }) => (
          <button
            key={key}
            type="button"
            onClick={() => setActive(key)}
            className={`flex items-center justify-center gap-1.5 rounded-md px-2 py-1.5 text-xs transition ${
              active === key
                ? "bg-blue-600 text-white"
                : "bg-white/10 text-neutral-300 hover:bg-white/15"
            }`}
          >
            <span className={`h-2 w-2 rounded-full ${dot}`} />
            {t(labelKey)}
          </button>
        ))}
      </div>

      <AdjustmentSlider
        label={t("editor.selectiveColor.hue")}
        value={current.hue}
        min={-30}
        max={30}
        onChange={(hue) => updateChannel({ hue })}
      />
      <AdjustmentSlider
        label={t("editor.selectiveColor.saturation")}
        value={current.saturation}
        min={-100}
        max={100}
        onChange={(saturation) => updateChannel({ saturation })}
      />
      <AdjustmentSlider
        label={t("editor.selectiveColor.luminance")}
        value={current.luminance}
        min={-100}
        max={100}
        onChange={(luminance) => updateChannel({ luminance })}
      />

      <div className="flex justify-end pt-1">
        <button
          type="button"
          onClick={resetTab}
          disabled={isDefault}
          className="rounded-md bg-white/10 px-2.5 py-1 text-xs text-neutral-300 transition hover:bg-white/15 disabled:opacity-40"
        >
          {t("editor.reset")}
        </button>
      </div>
    </EditorSection>
  );
}
