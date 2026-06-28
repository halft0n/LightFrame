import type { EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

interface DetailSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

export function DetailSection({ params, onChange }: DetailSectionProps) {
  const { t } = useTranslation();

  return (
    <EditorSection title={t("editor.detail")} defaultOpen={false}>
      <AdjustmentSlider
        label={t("editor.sharpness")}
        value={params.sharpness}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(sharpness) => onChange({ sharpness })}
      />
      <AdjustmentSlider
        label={t("editor.definition")}
        value={params.definition}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(definition) => onChange({ definition })}
      />
      <AdjustmentSlider
        label={t("editor.noiseReduction")}
        value={params.noiseReduction}
        min={0}
        max={100}
        defaultValue={0}
        onChange={(noiseReduction) => onChange({ noiseReduction })}
      />
    </EditorSection>
  );
}
