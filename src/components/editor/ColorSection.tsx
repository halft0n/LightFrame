import type { EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

interface ColorSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

export function ColorSection({ params, onChange }: ColorSectionProps) {
  const { t } = useTranslation();

  return (
    <EditorSection title={t("editor.color")}>
      <AdjustmentSlider
        label={t("editor.saturation")}
        value={params.saturation}
        min={-100}
        max={100}
        onChange={(saturation) => onChange({ saturation })}
      />
      <AdjustmentSlider
        label={t("editor.vibrance")}
        value={params.vibrance}
        min={-100}
        max={100}
        onChange={(vibrance) => onChange({ vibrance })}
      />
      <AdjustmentSlider
        label={t("editor.warmth")}
        value={params.warmth}
        min={-100}
        max={100}
        onChange={(warmth) => onChange({ warmth })}
      />
      <AdjustmentSlider
        label={t("editor.tint")}
        value={params.tint}
        min={-100}
        max={100}
        onChange={(tint) => onChange({ tint })}
      />
    </EditorSection>
  );
}
