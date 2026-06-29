# LightFrame / 影迹

[![Build Status](https://github.com/halft0n/CatchLight/actions/workflows/build.yml/badge.svg)](https://github.com/halft0n/CatchLight/actions/workflows/build.yml)
[![Latest Release](https://img.shields.io/github/v/release/halft0n/CatchLight?include_prereleases&label=release)](https://github.com/halft0n/CatchLight/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)](https://github.com/halft0n/CatchLight/releases)
[![Language](https://img.shields.io/badge/backend-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![Framework](https://img.shields.io/badge/framework-Tauri%202-blue?logo=tauri)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

> 拾一束光，留一段时光。 Catch the light, keep the moment.

LightFrame 是一款跨平台（Windows + Linux + macOS）、本地优先的照片查看与管理软件。不复制、不导入，以 Everything 级极速索引就地浏览用户现有文件夹中的照片与视频，并提供去重、截图识别、AI 相似搜索等智能整理能力。

## Screenshots

> Demo screenshots will be added before v0.1.0-beta release.
>
> Placeholder: [gallery](docs/screenshots/gallery.png) · [timeline](docs/screenshots/timeline.png) · [viewer](docs/screenshots/viewer.png)

## Features

- **macOS Photos-style UI** — timeline, locations, albums, people, folder tree
- **Lightning-fast indexing** — recursive walkdir + inotify file watching
- **No import/export required** — folders are your library, zero migration cost
- **Full-text search** — SQLite FTS5 filename and metadata search
- **Smart deduplication** — BLAKE3 exact + DHash/PHash perceptual matching with LSH
- **Similar photos** — CLIP embedding cosine similarity (optional ONNX)
- **Face detection** — SCRFD + ArcFace framework (optional ONNX)
- **Screenshot detection** — rule engine with EXIF and visual scoring
- **Non-destructive editor** — curves, levels, selective color adjustments
- **Map view** — Leaflet map with location clustering
- **Memories & On This Day** — timeline grouping and anniversary highlights
- **Batch operations** — delete, favorite, export, add to album (up to 1000 items)
- **Cross-platform** — Windows 10/11, Linux (Ubuntu 22.04+, Fedora 38+), macOS
- **Multi-language** — 简体中文, English
- **Privacy-first** — all local processing, no cloud uploads ([Privacy Policy](docs/PRIVACY.md))

## Keyboard Shortcuts

| Context | Shortcut | Action |
|---------|----------|--------|
| Photo grid | `Escape` | Clear selection |
| Photo grid | `Ctrl`/`Cmd` + `A` | Select all |
| Photo grid | `Ctrl`/`Cmd` + click | Toggle item selection |
| Photo grid | `Shift` + click | Range selection |
| Photo grid | `Delete` / `F` | Delete / favorite selection |
| Photo viewer | `←` / `→` | Previous / next photo |
| Photo viewer | `R` / `Shift+R` | Rotate clockwise / counter-clockwise |
| Photo viewer | `Escape` | Close viewer |
| Photo viewer | Scroll wheel | Zoom in / out |
| Video player | `Space` | Play / pause |
| Video player | `←` / `→` | Seek −5s / +5s |
| Image editor | `Ctrl+Z` | Undo |
| Image editor | `Ctrl+Shift+Z` | Redo |

See [User Guide](docs/USER_GUIDE.md) for the complete shortcut reference.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Vite, TailwindCSS v4 |
| Desktop Shell | Tauri 2.x |
| Backend | Rust (Cargo Workspace) |
| Database | SQLite + FTS5 (read/write connection split) |
| Mapping | Leaflet (react-leaflet) |
| AI Extension | ONNX Runtime (optional) + Python sidecar |

## Project Structure

```
CatchLight/
├── src/                    # React frontend
├── src-tauri/              # Tauri main crate
├── crates/                 # Rust domain crates
│   ├── catchlight-core/
│   ├── catchlight-db/
│   ├── catchlight-indexer/
│   ├── catchlight-metadata/
│   ├── catchlight-thumbnail/
│   ├── catchlight-dedup/
│   ├── catchlight-ai/
│   ├── catchlight-geo/
│   └── catchlight-video/
├── python/                 # Optional AI sidecar
├── locales/                # i18n translation files
└── docs/                   # Design documents
```

## Development

### Prerequisites

- Node.js >= 22
- pnpm >= 9
- Rust >= 1.77 (via [rustup](https://rustup.rs))
- **Linux:** `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`
- **Optional AI:** ONNX models or Python 3.10+

### Getting Started

```bash
# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

### Running Tests

```bash
pnpm test                    # frontend (Vitest)
cargo test --workspace       # Rust workspace
cargo fmt --check            # formatting
cargo clippy --workspace -- -D warnings
npx tsc --noEmit             # TypeScript type check
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Changelog](CHANGELOG.md) | Release history |
| [User Guide](docs/USER_GUIDE.md) | Installation, shortcuts, troubleshooting |
| [Beta Roadmap](docs/BETA_ROADMAP.md) | v0.1.0-beta release plan |
| [Privacy Policy](docs/PRIVACY.md) | Data handling and privacy |
| [Architecture](docs/3-architecture.md) | System architecture design |
| [Development Plan](docs/5-development-plan.md) | Phased development roadmap |

## Contributing

Contributions are welcome! Please:

1. Fork the repository and create a feature branch
2. Follow [conventional commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, etc.)
3. Run all tests locally before opening a PR:
   ```bash
   cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
   pnpm test && npx tsc --noEmit
   ```
4. Keep changes focused — one feature or fix per PR
5. Update docs when behavior changes

Bug reports and feature requests: [GitHub Issues](https://github.com/halft0n/CatchLight/issues)

## Acknowledgements

LightFrame 的诞生离不开以下优秀的开源项目：

| Project | Usage |
|---------|-------|
| [Tauri](https://tauri.app) | Cross-platform desktop framework |
| [React](https://react.dev) | Frontend UI library |
| [Vite](https://vite.dev) | Frontend build tool |
| [Tailwind CSS](https://tailwindcss.com) | Utility-first CSS framework |
| [SQLite](https://sqlite.org) | Embedded database engine |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite bindings for Rust |
| [image](https://github.com/image-rs/image) | Rust image processing library |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | Fast cryptographic hash for deduplication |
| [kamadak-exif](https://github.com/kamadak/exif-rs) | EXIF metadata extraction |
| [reverse_geocoder](https://github.com/gx0r/rrgeo) | Offline reverse geocoding |
| [Leaflet](https://leafletjs.com) | Interactive map for location browsing |
| [iPhotron](https://github.com/OliverZhaohaibin/iPhotron-LocalPhotoAlbumManager) | UI/UX design inspiration |
| [@fontsource/inter](https://fontsource.org/fonts/inter) | Self-hosted Inter font |
| [@tanstack/react-virtual](https://tanstack.com/virtual) | Virtual scrolling for large lists |

## Star History

<p align="center">
  <a href="https://www.star-history.com/#halft0n/CatchLight&type=Date">
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=halft0n/CatchLight&type=Date" />
  </a>
</p>

## License

MIT License. See [LICENSE](LICENSE) for details.
