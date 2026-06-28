import { Sidebar } from "@/components/layout/Sidebar";
import { MainContent } from "@/components/layout/MainContent";
import { useTranslation } from "@/i18n/useTranslation";

export default function App() {
  const { t } = useTranslation();

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 flex flex-col overflow-hidden">
        <header className="flex items-center justify-between px-6 py-3 border-b border-neutral-800">
          <h1 className="text-lg font-semibold">{t("app.title")}</h1>
        </header>
        <MainContent />
      </main>
    </div>
  );
}
