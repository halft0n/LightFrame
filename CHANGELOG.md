# Changelog

All notable changes to LightFrame will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (nothing yet)

## [0.0.19] - 2026-07-01

### Added
- **Person Groups**: `person_groups` table (v16 migration), CRUD operations, drag-and-drop grouping UI in PeopleView
- **Sidebar Pinning**: Pin albums, people, and smart albums to sidebar (max 10); persistent `settings` table; transactional pin/unpin; context menus
- **CLI Tool**: New `lightframe-cli` binary (`status`, `scan`, `export`, `dedup` subcommands); DB-aware `dedup --delete`
- **German i18n**: Complete `de.json` translation (394+ keys); 3-locale framework with runtime switching; locale picker in Settings
- **Facts vs Choices**: `rebuild_cache` with persistent staging tables; selective preservation of favorites, albums, manual faces, edit parameters; background post-scan restore; startup crash recovery

### Changed
- `rebuild_cache` now triggers automatic rescan and waits for completion before restoring user choices
- Pin operations use `BEGIN IMMEDIATE` transactions to prevent race conditions
- Person group commands validate group membership and existence
- CLI `dedup --delete` synchronizes deletions with the database

### Tests
- Rust: 737 tests (up from 683)
- Frontend: 639 tests (up from 636)
- Total: 1376

## [0.0.18] - 2026-07-01

### Added
- **Phase 5a**: Micro-first thumbnail loading, Filmstrip strip navigation, Live Photo support (paired MOV detection + playback)
- **Phase 5b**: AI model download manager, parallel scan pipeline (0.7×CPU cores), database reset command, dhash/phash integer-to-hex migration
- **Phase 5c**: Long-press preview popup, manual face annotation, memory-aware thumbnail budget, video trim with FFmpeg

### Fixed
- Scroll-intent ref lifecycle in PhotoGrid (useScrollIntent hook)
- Memory pressure detection using `is_under_pressure` for live state
- Code review fixes across all phases (validation, error handling, test coverage)

### Tests
- Rust: 683 tests (up from 643)
- Frontend: 636 tests (up from 561)
- Total: 1319

## [0.0.17] - 2026-07-01

### Fixed
- Windows image display: fix double URL encoding in custom protocol URLs
- Face protocol cache bypass: move security validation before cache lookup
- PhotoGrid first-render layout: defer grid rendering until container width is measured
- Folder removal: immediately refresh media list after removing a watched folder
- AiSettings: fix cancelled-download error suppression checking localized string
- Placeholder thumbnail caching: use `no-cache` header to prevent permanent caching

### Added
- Face cache invalidation on media delete, re-detect, and thumbnail regeneration
- `face_cache_dir` field in AppState for configurable face cache location
- Comprehensive model download tests (cancellation, cleanup, error paths, unknown filename)
- PhotoGrid layout tests (zero-width prevention, measured-width rendering)
- FolderManager test for media refresh after folder removal
- AiSettings tests (cancel button, error display, cancelled-error suppression)
- `cancelDownload` frontend wrapper test

### Security
- Face protocol: validate source path **before** serving cached face crops (prevents symlink bypass)

### Tests
- Rust: 643 tests (up from 630)
- Frontend: 561 tests (up from 554)
- Total: 1204

## [0.0.16] - 2026-06-30

### Fixed
- Clear stale hashes on reindex to prevent phantom duplicates
- Re-enrich skipped files on full rescan
- Normalize Windows `\\?\` extended path prefix in all DB operations

## [0.0.15] - 2026-06-30

### Fixed
- Code review fixes: macOS Intel compatibility
- Development workflow improvements

## [0.0.14] - 2026-06-29

### Added
- Producer-consumer event queue for scan workflow (replaces polling)
- Two-phase scan progress (indexed → complete)
- Sidebar navigation closes viewer on scan start
- Incremental media refresh during scan

### Fixed
- Windows image display in WebView2 (platform-aware protocol URL construction)
- Windows path short-name mismatch in path validation

## [0.0.13] - 2026-06-29

### Added
- RAW image format support (CR2, NEF, ARW, DNG) with embedded preview extraction
- Settings page scroll and section ordering improvements

### Removed
- Tauri updater plugin (replaced with manual GitHub Releases check)

### Security
- Additional symlink escape prevention in file serving protocols

## [0.0.12] - 2026-06-29

### Fixed
- CI build failures on multiple platforms
- Windows portable build configuration
- Extended path prefix handling in path validation

### Added
- Windows portable executable in release artifacts

## [0.0.11] - 2026-06-28

### Added
- Comprehensive test coverage for DB, protocols, frontend views
- Edge-case tests for P1/P2 fixes and cross-feature scenarios
- Batch insert optimization for scan performance
- Protocol utility functions for shared logic

### Fixed
- P0/P1 code quality issues from code review
- `eq_ignore_ascii_case` in original_protocol path comparison
- Eliminated dead code and suppressed warnings
- Applied `cargo fmt` formatting consistently

## [0.0.10] - 2026-06-28

### Fixed
- CI build error: tracing `char` type in MFT module
- Filmstrip now context-aware (shows neighbors from current view)
- Mutex poison handling: all DB locks propagate errors instead of panicking
- `selectedSet` memoized in PhotoGrid to avoid unnecessary re-renders
- Hardcoded em dash `"—"` replaced with i18n key `common.notAvailable`
- Error messages enriched with context (media_id, path, folder)
- Batch operations capped at 1000 items to prevent performance issues
- `getFavoriteState` now logs errors instead of silently returning false

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
