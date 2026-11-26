<div align="center">

# Forge IDE

### A focused code workspace, with Gemini alongside you.

Native desktop builds for Windows, macOS, and Linux. Built with Rust and Tauri. No Electron.

[Website](https://umarerth.github.io/ForgeIDE/) · [Downloads](https://github.com/UmarErth/ForgeIDE/releases/latest) · [Report an issue](https://github.com/UmarErth/ForgeIDE/issues)

[![Build and publish releases](https://github.com/UmarErth/ForgeIDE/actions/workflows/release.yml/badge.svg)](https://github.com/UmarErth/ForgeIDE/actions/workflows/release.yml)
[![GPL-3.0 License](https://img.shields.io/badge/license-GPL--3.0-8ab4f8.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/core-Rust-b45b3e.svg)](https://www.rust-lang.org/)

</div>

---

Forge IDE brings your project, editor, and Gemini coding assistant into one calm, dark workspace. Open a folder, work directly with your files, search your project, and ask Gemini about the code you have open.

## Downloads

Get current installers and portable builds from [GitHub Releases](https://github.com/UmarErth/ForgeIDE/releases/latest).

| Platform | Release files |
| --- | --- |
| Windows x64 | Setup `.exe`, `.msi`, and portable `.exe` |
| macOS | `.dmg` installer and zipped `.app` |
| Linux x86_64 | `.AppImage` |

Release builds run on native Windows, macOS, and Ubuntu runners. Push a version tag such as `v0.2.0`, or run **Build and publish releases** from GitHub Actions with a release tag, to build all platforms and attach their downloads to a GitHub Release.

## What Forge can do

- Browse project folders and open, edit, and save text files.
- Search text across a project and view its Git status.
- Ask Gemini to explain or review the current file, or help plan a feature.
- Store the Gemini key using the operating system credential store.
- Use the same Rust desktop core on Windows, macOS, and Linux.

## Quick start

1. Download and launch the build for your operating system.
2. On first launch, enter a display name and your Gemini API key. Create a key at [Google AI Studio](https://aistudio.google.com/apikey).
3. Open a project folder and start working. Forge stores your key in Windows Credential Manager, macOS Keychain, or Linux Secret Service.

The Gemini key is sent to Google's Gemini API when you send a prompt. Forge does not include a shared API key.

## Build from source

### Requirements

- Rust 1.77.2 or later and the Tauri CLI v2.
- Node.js 22 or later with npm.
- Windows: Microsoft C++ Build Tools and WebView2.
- macOS: Xcode Command Line Tools.
- Linux: GTK 3, WebKitGTK 4.1, libsoup 3, AppIndicator, and the Tauri build packages. See `scripts/build-linux.sh` and `.github/workflows/release.yml` for the package list.

Install frontend dependencies once:

```sh
npm ci
```

Build on the target operating system:

```sh
# Windows
powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1

# Windows portable executable
powershell -ExecutionPolicy Bypass -File scripts/build-portable-windows.ps1

# macOS
bash scripts/build-macos.sh

# Linux
bash scripts/build-linux.sh
```

Build packages are written to `target/release/bundle/`; the Windows portable executable is written to `target/portable/Forge-IDE.exe`.

## Project structure

- `src/` — Rust desktop backend, file operations, secure profile storage, Gemini integration, project search, and Git status.
- `ui/` — editor workspace and app interface, bundled with Vite for the system WebView.
- `website/` — Forge IDE's dark project landing page, deployed through GitHub Pages.
- `.github/workflows/` — native platform builds, GitHub Releases, and Pages deployment.

Forge is an early-stage editor foundation. Syntax highlighting, language servers, integrated terminals, Git actions, extensions, and multiple open editor tabs are not implemented yet. Contributions are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Forge IDE is open source under the [GNU General Public License v3.0](LICENSE), as selected for this repository.
