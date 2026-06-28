import type { EditParams } from "@/lib/editParams";
import { useTranslation } from "@/i18n/useTranslation";
import { AdjustmentSlider } from "./AdjustmentSlider";
import { EditorSection } from "./EditorSection";

interface LightSectionProps {
  params: EditParams;
  onChange: (patch: Partial<EditParams>) => void;
}

export function LightSection({ params, onChange }: LightSectionProps) {
  const { t } = useTranslation();

  return (
    <EditorSection title={t("editor.light")}>
      <AdjustmentSlider
        label={t("editor.brilliance")}
        value={params.brilliance}
        min={-100}
        max={100}
        onChange={(brilliance) => onChange({ brilliance })}
      />
      <AdjustmentSlider
        label={t("editor.exposure")}
        value={params.exposure}
        min={-100}
        max={100}
        onChange={(exposure) => onChange({ exposure })}
      />
      <AdjustmentSlider
        label={t("editor.highlights")}
        value={params.highlights}
        min={-100}
        max={100}
        onChange={(highlights) => onChange({ highlights })}
      />
      <AdjustmentSlider
        label={t("editor.shadows")}
        value={params.shadows}
        min={-100}
        max={100}
        onChange={(shadows) => onChange({ shadows })}
      />
      <AdjustmentSlider
        label={t("editor.brightness")}
        value={params.brightness}
        min={-100}
        max={100}
        onChange={(brightness) => onChange({ brightness })}
      />
      <AdjustmentSlider
        label={t("editor.contrast")}
        value={params.contrast}
        min={-100}
        max={100}
        onChange={(contrast) => onChange({ contrast })}
      />
      <AdjustmentSlider
        label={t("editor.blackPoint")}
        value={params.blackPoint}
        min={-100}
        max={100}
        onChange={(blackPoint) => onChange({ blackPoint })}
      />
    </EditorSection>
  );
}
