# CatchLight v0.1.0-beta Roadmap

## Current Status (v0.0.10)

- ✅ Core photo management (import, browse, organize)
- ✅ Album management with cover photos
- ✅ Full-text search (FTS5)
- ✅ Perceptual deduplication (DHash + PHash + LSH)
- ✅ Similar photo detection
- ✅ Face detection framework (ONNX)
- ✅ CLIP embedding framework (ONNX)
- ✅ Screenshot classification
- ✅ Geo-reverse coding
- ✅ Basic image editing (crop, rotate, filters)
- ✅ Batch export
- ✅ Keyboard shortcuts
- ✅ Timeline view ("On this day")
- ✅ Favorites system
- ✅ Soft delete + permanent delete
- ✅ File watcher (real-time)
- ✅ Database read/write split
- ✅ LSH-based dedup optimization

## Remaining for v0.1.0-beta

### Must-Have (P0)

- [ ] CLIP model auto-download with progress bar
- [ ] Semantic search fully functional
- [ ] Face clustering UI (view/merge/split persons)
- [ ] Windows code signing (remove SmartScreen warning)
- [ ] macOS .dmg packaging
- [ ] Auto-updater with signature verification

### Should-Have (P1)

- [ ] Thumbnail regeneration for corrupt/missing thumbnails
- [ ] RAW file support (via dcraw/libraw)
- [ ] HEIC/AVIF support
- [ ] Map view for geo-tagged photos
- [ ] Slideshow mode
- [ ] Print/share integration

### Nice-to-Have (P2)

- [ ] Cloud sync (WebDAV/S3)
- [ ] Mobile companion app
- [ ] Plugin system
- [ ] Advanced editing (layers, masks)
- [ ] Video timeline editor

## Release Criteria

- [ ] All tests passing on Windows, macOS, Linux
- [ ] <3s cold start time
- [ ] <100MB memory usage for 10K photos
- [ ] No known P0 bugs
- [ ] User guide complete
- [ ] Installer tested on clean Windows 10/11
