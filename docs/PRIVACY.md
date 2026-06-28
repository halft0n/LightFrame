# Privacy Policy

CatchLight is a **local-first** photo manager. Your photos, videos, and metadata never leave your device unless you explicitly export or share them.

## Data storage

All indexed data — file paths, thumbnails, EXIF metadata, albums, and search indexes — is stored in a SQLite database on your computer. CatchLight does not operate any cloud backend or sync service.

## No telemetry

CatchLight does not collect usage analytics, crash reports, or personal information. There are no tracking pixels, no account system, and no phone-home behavior.

## AI features

Optional AI capabilities (screenshot detection, face recognition, and future ONNX-based models) run **entirely on your machine**. The optional Python sidecar communicates with the desktop app over a local JSON-RPC channel — no data is sent to external AI services.

## Updates

The built-in auto-updater checks GitHub Releases for new versions. This request only fetches version metadata; it does not transmit your photo library, file paths, or any identifying information.

## Network usage

The only network activity is:

- **Map tiles** (optional) — Leaflet may load map tiles from OpenStreetMap when you use the location view
- **Update checks** — as described above

You can use CatchLight fully offline except for map tiles and update checks.

## Your control

Because everything is local, you control your data completely. Uninstalling CatchLight removes the application; your original photo files are never modified or deleted unless you explicitly choose to do so within the app.
