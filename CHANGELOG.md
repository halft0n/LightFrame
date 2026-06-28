# Changelog

All notable changes to CatchLight will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (nothing yet)

## [0.0.9] - 2026-06-28

### Added
- CLIP ONNX image encoding (optional `clip` feature)
- Cosine similarity search for similar photos
- `SimilarPhotosPanel` slide-in UI in photo viewer
- Face detection/clustering pipeline (optional `face` feature)
- Person management: rename, merge, cluster faces
- AI model management UI (`AiSettings.tsx`) with status dashboard
- Screenshot sub-classification (code/chat/document/game/web)
- `ScreenshotView` with category chip filter
- Semantic search mode toggle in search bar
- `SearchResultsView` with AI/Text mode indicator
- Criterion performance benchmarks for DB and dedup crates
- Bilingual user guide (`docs/USER_GUIDE.md`)
- Code signing documentation (`docs/SIGNING.md`)
- MFT/USN fast indexing framework (Windows, conditional compilation)
- CI tag-vs-config version verification step

### Security
- Fix symlink escape: canonicalize paths before serving in `original://` protocol
- Fix path traversal in batch export: sanitize filenames
- Add 500MB size limit for original file serving (prevent OOM)
- Add 64KB limit for edit parameter payloads

### Fixed
- Windows thumbnail loading: CORS headers, robust URI parsing, path normalization
- NSIS installer: add SimpChinese/English languages, proper config
- Version sync: unified to 0.0.9 across Cargo/Tauri/npm
- Delete confirmation dialogs in PhotoViewer and PhotoGrid
- Error handling in appStore loadMedia/loadMoreMedia
- Scan optimization: skip unchanged files by (size, mtime) check
- `create_smart_album` deadlock (mutex held across DB calls)
- AI status module name check

### Tests (64 new frontend + 25 new Rust)
- App routing and responsive layout tests
- InfoPanel, PhotoViewer keyboard shortcuts, MemoriesView tests
- FolderView, Sidebar ARIA, SelectionToolbar, dragMedia tests
- useMediaQuery hook tests
- Screenshot detection edge cases (ultrawide, tablet, threshold)
- Keyset pagination, batch ops, smart album counts, concurrent reads
- PHash/DHash stability and distance tests
- Thumbnail size path generation tests
- Cosine similarity and find_similar unit tests

### Fixed
- `create_smart_album` deadlock (mutex held across DB calls)
- AI status module name check corrected

## [0.0.8] - 2026-06-28

### Added
- CSS loading and transition animations (fadeIn, slideInRight, scaleIn)
- Responsive layout with mobile sidebar hamburger menu
- ARIA accessibility attributes across all major components
- Focus-visible keyboard navigation styles
- Viewer adjacent image preloading (±2 neighbors)
- `useImagePreloader` hook for background image prefetching
- `useMediaQuery` hook for responsive breakpoints

### Improved
- Photo grid adapts to minimum 2 columns on narrow screens
- Sidebar uses `aria-current="page"` for active navigation
- PhotoViewer uses `role="dialog"` and `aria-modal` semantics
- PhotoCard uses `role="gridcell"` with `aria-selected` state

## [0.0.7] - 2026-06-28

### Added
- EXIF info panel in photo viewer (I key toggle)
- Drag-and-drop photos to album sidebar
- LRU thumbnail memory cache (2000 micro / 500 small+large)
- Keyboard shortcuts (Delete, F, I, E in viewer; Delete, F, Ctrl+A, Esc in grid)

### Fixed
- CI artifact collection paths for GitHub releases
- Deadlock in `create_album` database operation
- Scroll listener option mismatch in PhotoGrid
- Virtualizer stale row heights on thumbnail size change

## [0.0.6] - 2026-06-28

### Added
- PHash perceptual deduplication (DCT-based 64-bit hash)
- "On This Day" memories feature
- Album editing (rename, set cover, remove items)
- CI dual-platform testing (Ubuntu + Windows)

## [0.0.5] - 2026-06-28

### Added
- Folder tree browsing in sidebar
- Thumbnail size control (3 presets: Small/Medium/Large)
- Search history (last 10 searches)
- Photo viewer rotation (`R` / `Shift+R` shortcuts)
- Batch export to folder
- Python AI sidecar JSON-RPC framework
- Enhanced screenshot detection with EXIF rules and visual feature scoring
- SQL performance indexes (15 new indexes)
- Keyset pagination for efficient large dataset browsing
- SQLite PRAGMA optimizations (64MB cache, 256MB mmap)
- Watcher now handles file deletions (soft-delete) and renames (path update)

### Fixed
- `original://` protocol security: path allowlist based on watched folders
- `get_media_by_id` now filters out soft-deleted items
- Permanent delete now actually removes files from disk + thumbnail cache
- Removed double disk-delete between repo and commands layer

### Security
- Added path validation for `original://` protocol to prevent arbitrary file access

## [0.0.4] - 2026-06-28

### Added
- Complete non-destructive image editor with curves, levels, selective color
- `is_favorite` backend command
- Frontend error handling with try/catch on all IPC calls
- Permanent deletion confirmation dialog
- Light/dark theme contrast improvements

### Fixed
- Range selection in VideosView using correct context items
- PersonDetailView pagination logic
- Dialog save permission for image export
- Invalid HTML nesting in album/smart album list views
- AlbumDetailView infinite scrolling
- ImageEditor export state management
- Timer memory leak in ImageEditor
- Deep merge for nested edit parameters
- Theme settings subtitle text

## [0.0.3] - 2026-06-28

### Added
- Initial release with core features
- File indexing with walkdir + inotify watching
- Three-level thumbnail system (micro/small/large) with `thumb://` protocol
- Virtual scrolling photo grid with multi-select
- Timeline view with date grouping
- Full-screen photo viewer with zoom/pan
- BLAKE3 exact deduplication
- DHash perceptual deduplication
- FTS5 full-text search
- Leaflet map view with location clustering
- Album management (CRUD + media association)
- Smart albums with rule-based filtering
- Memories generation (date+location clustering)
- Favorites and soft-delete with 30-day cleanup
- Batch operations (delete, favorite, add to album, restore)
- Video playback support with FFmpeg thumbnails
- Light/dark theme with system preference
- Chinese/English internationalization
