# Windows Code Signing for LightFrame

LightFrame release builds are currently **unsigned**. This document explains how to set up Authenticode signing for the Tauri desktop app in local builds and CI.

## Why Sign?

- Reduces **Windows SmartScreen** "Unknown publisher" warnings
- Lowers false positives from antivirus products (e.g. 360 Security) on WebView2-based apps
- Gives users confidence that installers and binaries have not been tampered with

## 1. Obtain an Authenticode Certificate

Purchase a code-signing certificate from a trusted Certificate Authority (CA), for example:

- [DigiCert](https://www.digicert.com/signing/code-signing-certificates)
- [Sectigo](https://www.sectigo.com/ssl-certificates-tls/code-signing)

**Standard (OV)** certificates work for signing. For faster SmartScreen reputation, consider an **Extended Validation (EV)** certificate on a hardware token.

Export or convert the certificate to a format usable by Tauri (typically a `.pfx` / PKCS#12 file containing the private key and certificate chain).

## 2. Tauri Signing Keys

Tauri 2 uses updater signing keys separate from Authenticode. Generate a key pair once:

```bash
pnpm tauri signer generate -w ~/.tauri/lightframe.key
```

This creates:

- **Private key** — keep secret; used to sign update artifacts
- **Public key** — embed in `tauri.conf.json` under `plugins.updater.pubkey`

### Environment variables

Set these when building signed releases (locally or in CI):

| Variable | Description |
|----------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of the private key file, or path to the key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key (if encrypted) |

Example (local shell):

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/lightframe.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-key-password"
pnpm tauri build
```

Tauri reads these during `tauri build` to sign update bundles. See the official docs: [Tauri — Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/) and [Tauri — Updater Signing](https://v2.tauri.app/plugin/updater/#signing-updates).

## 3. Authenticode Signing (Windows binaries)

After `pnpm tauri build`, sign the executable and installer with `signtool` (Windows SDK):

```powershell
# Main application binary
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "src-tauri\target\release\lightframe.exe"

# NSIS installer (sign after bundling)
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "src-tauri\target\release\bundle\nsis\LightFrame_*-setup.exe"
```

Use `/f your-cert.pfx /p cert-password` instead of `/a` when selecting a specific certificate.

## 4. GitHub Actions Secrets

Add the following repository secrets (Settings → Secrets and variables → Actions):

| Secret | Value |
|--------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | Full private key text (multiline) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key password, or empty string if none |
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` file (optional, for Authenticode) |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX password (optional) |

Example workflow snippet:

```yaml
- name: Build Tauri app
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
  run: pnpm tauri build
```

Decode and apply Authenticode signing in a subsequent Windows-only step when `WINDOWS_CERTIFICATE` is configured.

## 5. macOS and Linux

- **macOS**: Developer ID signing and Apple notarization are required for smooth Gatekeeper behavior. See [Tauri — macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/).
- **Linux**: `.deb`, `.rpm`, and `.AppImage` packages generally do not require code signing.

## References

- [Tauri v2 — Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri v2 — macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri Updater — Signing updates](https://v2.tauri.app/plugin/updater/#signing-updates)
- [Microsoft signtool documentation](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)
