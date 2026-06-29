# Windows Code Signing & Auto-Updater for LightFrame

LightFrame release builds are currently **unsigned** (Authenticode). This document explains how to set up code signing and the Tauri auto-updater for local builds and CI.

## Why Sign?

- Reduces **Windows SmartScreen** "Unknown publisher" warnings
- Lowers false positives from antivirus products (e.g. 360 Security) on WebView2-based apps
- Gives users confidence that installers and binaries have not been tampered with
- Enables secure in-app updates via the Tauri updater plugin

## 1. Obtain an Authenticode Certificate (Optional)

Purchase a code-signing certificate from a trusted Certificate Authority (CA), for example:

- [DigiCert](https://www.digicert.com/signing/code-signing-certificates)
- [Sectigo](https://www.sectigo.com/ssl-certificates-tls/code-signing)

**Standard (OV)** certificates work for signing. For faster SmartScreen reputation, consider an **Extended Validation (EV)** certificate on a hardware token.

Export or convert the certificate to a format usable by Tauri (typically a `.pfx` / PKCS#12 file containing the private key and certificate chain).

## 2. Tauri Updater Signing Keys

Tauri 2 uses updater signing keys separate from Authenticode. Generate a key pair once:

```bash
pnpm tauri signer generate -w ~/.tauri/lightframe.key
```

This creates:

- **Private key** (`~/.tauri/lightframe.key`) — keep secret; used to sign update artifacts
- **Public key** (`~/.tauri/lightframe.key.pub`) — embed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`

The public key is already configured in `tauri.conf.json`. If you regenerate keys, update the `pubkey` field with the contents of the `.pub` file.

### Environment variables

Set these when building signed releases (locally or in CI):

| Variable | Description |
|----------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of the private key file, or path to the key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key (empty string if none) |

Example (local shell):

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/lightframe.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
pnpm tauri build
```

Tauri reads these during `tauri build` to sign update bundles (`.sig` files). See the official docs: [Tauri — Updater Signing](https://v2.tauri.app/plugin/updater/#signing-updates).

## 3. GitHub Actions Secrets

Add the following repository secrets (Settings → Secrets and variables → Actions):

| Secret | Value |
|--------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | Full private key text (multiline) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key password, or empty string if none |
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` file (optional, for Authenticode) |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX password (optional) |

The build workflow (`.github/workflows/build.yml`) passes the Tauri signing secrets to `pnpm tauri build` on tag releases. After all platform builds complete, the release job generates `latest.json` pointing to the signed updater bundles on GitHub Releases.

### Updater endpoint

The app checks for updates at:

```
https://github.com/halft0n/LightFrame/releases/latest/download/latest.json
```

Each release includes platform-specific updater bundles (`.nsis.zip`, `.app.tar.gz`, `.AppImage.tar.gz`) and their `.sig` signature files.

## 4. Authenticode Signing (Windows binaries)

After `pnpm tauri build`, sign the executable and installer with `signtool` (Windows SDK):

```powershell
# Main application binary
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "src-tauri\target\release\lightframe.exe"

# NSIS installer (sign after bundling)
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "src-tauri\target\release\bundle\nsis\LightFrame_*-setup.exe"
```

Use `/f your-cert.pfx /p cert-password` instead of `/a` when selecting a specific certificate.

## 5. macOS and Linux

- **macOS**: Developer ID signing and Apple notarization are required for smooth Gatekeeper behavior. See [Tauri — macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/).
- **Linux**: `.deb`, `.rpm`, and `.AppImage` packages generally do not require code signing.

## References

- [Tauri v2 — Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri v2 — macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri Updater — Signing updates](https://v2.tauri.app/plugin/updater/#signing-updates)
- [Microsoft signtool documentation](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
