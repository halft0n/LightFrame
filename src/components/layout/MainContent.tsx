import { useTranslation } from "@/i18n/useTranslation";
import { PhotoGrid } from "@/components/gallery/PhotoGrid";
import { FolderManager } from "@/components/settings/FolderManager";
import { TimelineView } from "@/components/timeline/TimelineView";
import { LocationView } from "@/components/locations/LocationView";
import { DedupView } from "@/components/dedup/DedupView";
import { ScreenshotView } from "@/components/screenshots/ScreenshotView";
import { AlbumListView } from "@/components/albums/AlbumListView";
import { AlbumDetailView } from "@/components/albums/AlbumDetailView";
import { FavoritesView } from "@/components/favorites/FavoritesView";
import { DeletedView } from "@/components/deleted/DeletedView";
import { SearchResults } from "@/components/search/SearchResults";
import { PhotoViewer } from "@/components/viewer/PhotoViewer";
import { ViewerOverlay } from "@/components/viewer/ViewerOverlay";
import { useAppStore } from "@/store/appStore";

function ViewerLayer() {
  const { viewingMediaId } = useAppStore();
  if (viewingMediaId == null) return null;
  return (
    <ViewerOverlay>
      <PhotoViewer mediaId={viewingMediaId} />
    </ViewerOverlay>
  );
}

export function MainContent() {
  const { t } = useTranslation();
  const { currentView, watchedFolders, searchQuery } = useAppStore();

  if (searchQuery.trim()) {
    return (
      <>
        <SearchResults />
        <ViewerLayer />
      </>
    );
  }

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

  if (currentView === "settings") return <FolderManager />;

  if (currentView === "duplicates") {
    return (
      <>
        <DedupView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "screenshots") {
    return (
      <>
        <ScreenshotView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "all") {
    return (
      <>
        <PhotoGrid />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "timeline") {
    return (
      <>
        <TimelineView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "locations") {
    return (
      <>
        <LocationView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "albums") {
    return (
      <>
        <AlbumListView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "album-detail") {
    return (
      <>
        <AlbumDetailView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "favorites") {
    return (
      <>
        <FavoritesView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "deleted") {
    return (
      <>
        <DeletedView />
        <ViewerLayer />
      </>
    );
  }

  return (
    <div className="flex flex-1 items-center justify-center text-neutral-500">
      <p className="text-sm">{t("main.welcome")}</p>
    </div>
  );
}
