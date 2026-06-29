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
import { SearchResultsView } from "@/components/search/SearchResultsView";
import { FolderView } from "@/components/folders/FolderView";
import { MapView } from "@/components/map/MapView";
import { SlideshowView } from "@/components/viewer/SlideshowView";
import { PhotoViewer } from "@/components/viewer/PhotoViewer";
import { useAppStore } from "@/store/appStore";

export function MainContent() {
  const { t } = useTranslation();
  const { currentView, watchedFolders, searchQuery, viewingMediaId, slideshowActive } = useAppStore();

  if (slideshowActive) {
    return <SlideshowView />;
  }

  if (viewingMediaId != null) {
    return <PhotoViewer mediaId={viewingMediaId} />;
  }

  if (searchQuery.trim()) {
    return <SearchResultsView />;
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

  if (currentView === "duplicates") return <DedupView />;

  if (currentView === "screenshots") return <ScreenshotView />;

  if (currentView === "all") return <PhotoGrid />;

  if (currentView === "videos") return <VideosView />;

  if (currentView === "timeline") return <TimelineView />;

  if (currentView === "locations") return <LocationView />;

  if (currentView === "map") return <MapView />;

  if (currentView === "albums") return <AlbumListView />;

  if (currentView === "album-detail") return <AlbumDetailView />;

  if (currentView === "smart-albums") return <SmartAlbumListView />;

  if (currentView === "smart-album-detail") return <SmartAlbumView />;

  if (currentView === "memories") return <MemoriesView />;

  if (currentView === "memory-detail") return <MemoryDetailView />;

  if (currentView === "people") return <PeopleView />;

  if (currentView === "person-detail") return <PersonDetailView />;

  if (currentView === "favorites") return <FavoritesView />;

  if (currentView === "deleted") return <DeletedView />;

  if (currentView === "folder") return <FolderView />;

  return (
    <div className="flex flex-1 items-center justify-center text-neutral-500">
      <p className="text-sm">{t("main.welcome")}</p>
    </div>
  );
}
