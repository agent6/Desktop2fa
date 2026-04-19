# Desktop2FA

Desktop2FA is a compact Tauri desktop app for viewing and copying TOTP codes with minimal friction.

It uses:
- Tauri v2
- Rust for native logic and secret storage
- React + TypeScript + Vite for the UI

## Highlights

- Small desktop-first TOTP app
- Fast one-click copy flow
- Separate settings and account editor windows
- OTP URI import support
- Encrypted local secret vault with no macOS Keychain prompts
- Offline-first behavior

## Stack

- `src-tauri/`: Rust backend, Tauri window management, encrypted local vault, TOTP generation
- `src/`: React UI for the main app, settings list, and account editor

## Local Development

Requirements:
- Node.js 20+
- Rust stable
- Tauri system dependencies for your platform

Install dependencies:

```bash
npm install
```

Run the desktop app in development:

```bash
npm run tauri dev
```

Run frontend tests:

```bash
npm test
```

Build the frontend:

```bash
npm run build
```

Build the desktop app locally:

```bash
npm run tauri build
```

## Release Builds

This repo includes a GitHub Actions release workflow at [.github/workflows/release.yml](./.github/workflows/release.yml).

What it does:
- builds release artifacts for macOS, Windows, and Linux
- creates or updates a GitHub Release
- uploads Tauri bundles from each OS runner to that release

Trigger it by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow is based on the official Tauri GitHub pipeline guidance and `tauri-action`:
- [Tauri GitHub pipeline docs](https://v2.tauri.app/distribute/pipelines/github/)
- [tauri-apps/tauri-action](https://github.com/tauri-apps/tauri-action)

## Downloads

Latest release page:
- [Desktop2FA Releases](https://github.com/agent6/Desktop2fa/releases/latest)

Direct download links for the primary desktop installers:
- macOS DMG: [Desktop2FA-darwin-aarch64.dmg](https://github.com/agent6/Desktop2fa/releases/latest/download/Desktop2FA-darwin-aarch64.dmg)
- Windows MSI: [Desktop2FA-windows-x86_64.msi](https://github.com/agent6/Desktop2fa/releases/latest/download/Desktop2FA-windows-x86_64.msi)
- Linux AppImage: [Desktop2FA-linux-x86_64.AppImage](https://github.com/agent6/Desktop2fa/releases/latest/download/Desktop2FA-linux-x86_64.AppImage)

Additional release artifacts may also be present depending on the platform runner:
- Windows setup EXE: `Desktop2FA-windows-x86_64-setup.exe`
- Linux Debian package: `Desktop2FA-linux-x86_64.deb`

## GitHub Release Checklist

Before uploading this repository to GitHub:

1. Create the GitHub repository.
2. Push this project.
3. In the repository settings, allow GitHub Actions to have `Read and write permissions`.
4. Push a version tag such as `v0.1.0`.
5. Wait for the `release` workflow to finish.
6. Review the generated GitHub Release and publish it.

## Expected Release Artifacts

Depending on the runner and platform packaging support, the release will include Tauri bundles such as:

- macOS: `.app`, `.dmg`
- Windows: `.msi` and/or `.exe`
- Linux: `.AppImage`, `.deb`, and related bundles

## Security Note

Desktop2FA now stores secrets in an encrypted local vault instead of macOS Keychain. That removes repeated Keychain prompts, but it is a different security model than OS-native credential storage.
