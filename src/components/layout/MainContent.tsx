import { useTranslation } from "@/i18n/useTranslation";
import { PhotoGrid } from "@/components/gallery/PhotoGrid";
import { FolderManager } from "@/components/settings/FolderManager";
import { TimelineView } from "@/components/timeline/TimelineView";
import { PhotoViewer } from "@/components/viewer/PhotoViewer";
import { ViewerOverlay } from "@/components/viewer/ViewerOverlay";
import { useAppStore } from "@/store/appStore";

export function MainContent() {
  const { t } = useTranslation();
  const { currentView, watchedFolders, viewingMediaId } = useAppStore();

  if (watchedFolders.length === 0 && currentView !== "settings") {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <div className="space-y-4 text-center">
          <div className="text-6xl">📷</div>
          <p className="text-lg">{t("main.welcome")}</p>
          <p className="text-sm text-neutral-600">{t("main.addFolder")}</p>
        </div>
      </div>
    );
  }

  if (currentView === "settings") {
    return <FolderManager />;
  }

  if (currentView === "all") {
    return (
      <>
        <PhotoGrid />
        {viewingMediaId != null && (
          <ViewerOverlay>
            <PhotoViewer mediaId={viewingMediaId} />
          </ViewerOverlay>
        )}
      </>
    );
  }

  if (currentView === "timeline") {
    return (
      <>
        <TimelineView />
        {viewingMediaId != null && (
          <ViewerOverlay>
            <PhotoViewer mediaId={viewingMediaId} />
          </ViewerOverlay>
        )}
      </>
    );
  }

  return (
    <div className="flex flex-1 items-center justify-center text-neutral-500">
      <p className="text-sm">{t("main.welcome")}</p>
    </div>
  );
}
