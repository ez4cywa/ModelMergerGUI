<div align="center">

# Cast Model Merger GUI

**Drop CAST parts, merge them in groups, and inspect the result in independent 3D windows.**

A Rust-native Windows tool for model merging, previews and magazine filling.

[![Release](https://img.shields.io/github/v/release/ez4cywa/ModelMergerGUI)](https://github.com/ez4cywa/ModelMergerGUI/releases/latest)
[![Windows x64](https://img.shields.io/badge/Windows-x64-0078D4)](https://github.com/ez4cywa/ModelMergerGUI/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-native-dea584)](rust/Cargo.toml)
[![MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust CI](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml)

[简体中文](README.md) · English

[**Download portable edition**](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip) · [User guide](docs/user-guide.en.md) · [Release notes](https://github.com/ez4cywa/ModelMergerGUI/releases) · [Report an issue](https://github.com/ez4cywa/ModelMergerGUI/issues/new/choose)

</div>

![English main interface](docs/images/rust-native/main-window-en.png)

*Screenshots show the Rust-native interface. Some predate the latest release; current controls may differ.*

## What you can do

| Scenario | Capability |
| --- | --- |
| Combine separate parts | Merge 2–15 CAST parts per group with automatic or manual root selection |
| Process several models | Independent groups, two concurrent merges, queuing and cancellation |
| Inspect without merging | Drop multiple CAST files into the preview area to open independent windows |
| Check geometry and assembly | GPU depth rendering, rotation, zoom, ground grid and model information |
| Populate magazines | Place cartridges at ammunition bones; optionally copy layouts to spare magazines |
| Personalize the interface | Chinese, English, French, Russian and Spanish; persistent light/dark switching |

Model processing is local. Previews do not modify source files; merge output is written to a temporary file and read back for validation. Update checks are off by default and can be enabled or run manually in **Help → About**.

## Download and run

1. Download [**CastModelMerger-win-x64.zip**](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip).
2. Extract the entire ZIP and run `CastModelMerger.exe`, not from inside the archive.
3. Use your own `.cast` files to preview or merge models.

Requires Windows 10/11 x64 and a Direct3D 12-capable graphics driver for previews. **No .NET or Rust installation is required.** Releases provide a portable ZIP, not an installer.

The same [Release](https://github.com/ez4cywa/ModelMergerGUI/releases/latest) includes a SHA-256 checksum. Run `Get-FileHash .\CastModelMerger-win-x64.zip -Algorithm SHA256` in the download directory and compare its digest with the checksum file.

## Quick start

- **Preview**: drop one or more CAST files into the top preview area → rotate, zoom or inspect model information.
- **Merge**: add 2–15 parts to a group → check the root and output path → start merging → preview the result.
- **Fill magazines**: open **Fill magazine** → select a weapon and a cartridge with one `tag_ammo` bone → select targets → save to a new file.

### Where to drop files

| Drop target | Result |
| --- | --- |
| Top model preview area | Open independent previews without adding parts to groups |
| Existing group or part slot | Append parts to that group, up to 15 |
| Other areas | Put each batch in a separate group, reusing an idle empty group or creating one |

Drop targets highlight during dragging. You do not need to merge the previous batch before importing another. See the [full guide](docs/user-guide.en.md) for controls and filling rules.

<details>
<summary>Preview and magazine screenshots</summary>

| Light preview | Dark preview |
| --- | --- |
| ![Light preview](docs/images/rust-native/model-preview-zh.png) | ![Dark preview](docs/images/rust-native/model-preview-dark-zh.png) |

![Magazine filling, v2.2.1 Chinese interface](docs/images/rust-native/ammo-fill-zh.png)

*The magazine screenshot is from v2.2.1. Current versions consolidate duplicate controls into an inline Copy source selection.*

</details>

## Scope and limits

- Input and output use `.cast`. This is not a general format converter or a game asset extractor.
- Root detection, skeleton connection and repositioning follow upstream behavior; arbitrary parts are not guaranteed to assemble correctly.
- Previews use neutral geometry shading and display up to 250,000 triangles, with a notice when sampled. Merging uses the full data.
- Magazine filling requires supported bone names and unit-scale rigid skeletons, with limits of 512 slots and five million added vertices. Output must use a new filename.
- Everything is implemented natively in Rust; only Windows x64 packages are published. No macOS or Linux packages are published.

## Troubleshooting and local data

**No preview after dropping a file?** Only the top preview area opens previews; other areas import parts. Alternatively use **File → Open model preview…** (`Ctrl+O`).

**A window fails to open or a preview crashes?** Confirm full extraction and update the graphics driver. Include reproduction steps and diagnostic logs when reporting the issue:

```text
%LocalAppData%\CastModelMerger\logs\CastModelMerger.log
```

Logs contain version, GPU, model paths and preview lifecycle events. Rust panics include backtraces; native Windows exception codes and addresses are recorded on a best-effort basis. See the [guide](docs/user-guide.en.md) for concurrent logs and temporary-directory fallback. Logs remain local; redact private paths before sharing.

**Where are settings stored?** In `%LocalAppData%\CastModelMerger\settings.json`. Theme choices save automatically; use **Settings → Save settings** for language and other preferences. Selected model paths are not persisted.

## Run from source and verify

Development requires Windows x64, Rust 1.96 or a compatible newer stable toolchain, and the matching Windows linker tools. Run from the repository root:

```powershell
git clone https://github.com/ez4cywa/ModelMergerGUI.git
cd ModelMergerGUI
# Read the MiSans license, then download the font if you accept it
.\scripts\Install-MiSans.ps1 -AcceptLicense
cargo run --manifest-path rust/Cargo.toml -p model-merger-gui --bin CastModelMerger
```

```powershell
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace
# Produce the Windows x64 ZIP and SHA-256 checksum
.\scripts\Publish-RustNative.ps1
```

[Windows CI](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/rust-ci.yml) checks formatting, Clippy, tests, release builds and icon resources. [Decoder fuzzing](https://github.com/ez4cywa/ModelMergerGUI/actions/workflows/fuzz.yml) runs on a schedule. See the [build guide](docs/user-guide.en.md#build-and-test) for font licensing and build details.

## Structure and documentation

```text
rust/crates/cast-codec              CAST encoding/decoding and resource budgets
rust/crates/model-merger-engine     Merging, filling, assembly, output validation, preview sampling
rust/crates/model-merger-app-core   Workspace, settings, localization and scheduling
rust/crates/model-merger-gui        egui/wgpu desktop interface
rust/fuzz                          Decoder fuzzing
scripts/                           Font preparation, Windows builds and release checks
docs/                              Guides, design records and release notes
tests/fixtures/rust-migration      Golden corpora for the Rust merge tests
```

[English guide](docs/user-guide.en.md) · [中文指南](docs/user-guide.zh-CN.md) · [Rust migration (Chinese)](docs/full-rust-migration.md) · [Magazine bone research (Chinese)](docs/ammo-bone-research.md) · [Version notes](docs/release-notes)

## Contributing and license

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). Include the version, reproduction steps, expected and actual results, and relevant logs. Prefer a minimal model sample that can be shared publicly.

The original ModelMerger was developed by Philip / Scobalula, with Cast support added by [echo000](https://github.com/echo000/ModelMerger). Attribution is retained and source code uses the [MIT License](LICENSE). Embedded MiSans follows its own license; see [third-party notices](THIRD-PARTY-NOTICES.md).
