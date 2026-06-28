import { EmptyState } from "@/components/ui/EmptyState";
import { useTranslation } from "@/i18n/useTranslation";
import { PhotoGrid } from "@/components/gallery/PhotoGrid";
import { VideosView } from "@/components/gallery/VideosView";
import { FolderManager } from "@/components/settings/FolderManager";
import { TimelineView } from "@/components/timeline/TimelineView";
import { LocationView } from "@/components/locations/LocationView";
import { DedupView } from "@/components/dedup/DedupView";
import { ScreenshotView } from "@/components/screenshots/ScreenshotView";
import { AlbumListView } from "@/components/albums/AlbumListView";
import { AlbumDetailView } from "@/components/albums/AlbumDetailView";
import { FavoritesView } from "@/components/favorites/FavoritesView";
import { DeletedView } from "@/components/deleted/DeletedView";
import { SmartAlbumListView } from "@/components/smart-albums/SmartAlbumListView";
import { SmartAlbumView } from "@/components/smart-albums/SmartAlbumView";
import { MemoriesView } from "@/components/memories/MemoriesView";
import { MemoryDetailView } from "@/components/memories/MemoryDetailView";
import { PeopleView } from "@/components/people/PeopleView";
import { PersonDetailView } from "@/components/people/PersonDetailView";
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
      <EmptyState
        variant="welcome"
        title={t("main.welcome")}
        description={t("main.addFolder")}
      />
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

  if (currentView === "videos") {
    return (
      <>
        <VideosView />
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

  if (currentView === "smart-albums") {
    return (
      <>
        <SmartAlbumListView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "smart-album-detail") {
    return (
      <>
        <SmartAlbumView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "memories") {
    return (
      <>
        <MemoriesView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "memory-detail") {
    return (
      <>
        <MemoryDetailView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "people") {
    return (
      <>
        <PeopleView />
        <ViewerLayer />
      </>
    );
  }

  if (currentView === "person-detail") {
    return (
      <>
        <PersonDetailView />
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
