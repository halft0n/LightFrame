import type { EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

interface EffectsSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

export function EffectsSection({ params, onChange }: EffectsSectionProps) {
  const { t } = useTranslation();

  return (
    <EditorSection title={t("editor.effects")} defaultOpen={false}>
      <AdjustmentSlider
        label={t("editor.vignette")}
        value={params.vignette}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(vignette) => onChange({ vignette })}
      />
      <AdjustmentSlider
        label={t("editor.vignetteRadius")}
        value={params.vignetteRadius}
        min={0}
        max={100}
        defaultValue={50}
        onChange={(vignetteRadius) => onChange({ vignetteRadius })}
      />
      <AdjustmentSlider
        label={t("editor.grain")}
        value={params.grain}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(grain) => onChange({ grain })}
      />
      <AdjustmentSlider
        label={t("editor.bwIntensity")}
        value={params.bwIntensity}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(bwIntensity) => onChange({ bwIntensity })}
      />
      <AdjustmentSlider
        label={t("editor.bwTone")}
        value={params.bwTone}
        min={-100}
        max={100}
        onChange={(bwTone) => onChange({ bwTone })}
      />
    </EditorSection>
  );
}
