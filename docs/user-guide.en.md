# Cast Model Merger GUI User Guide

[简体中文](../README.md) | English

A native Windows desktop tool for `.cast` models, built with Rust, egui and wgpu. Merge model parts in independent groups, inspect them in standalone 3D previews, and fill magazines at ammunition bones. Based on the merging logic from [echo000/ModelMerger](https://github.com/echo000/ModelMerger).

[Download Windows x64 portable](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip) · [Release notes](https://github.com/ez4cywa/ModelMergerGUI/releases/latest) · [Usage guide](#usage) · [Build from source](#build-and-test) · [Report an issue](https://github.com/ez4cywa/ModelMergerGUI/issues)

- **Multi-group merging**: 2–15 parts per group, batch drag-and-drop, independent jobs, queuing and cancellation.
- **Model previews**: a dedicated drop zone opens multiple models, with rotation, zoom and model information.
- **Magazine filling**: place cartridge models at ammunition bones and optionally copy the layout to spare magazines.
- **Ready to run**: portable Windows x64 app, no .NET required; Chinese, English, French, Russian and Spanish, with one-click light/dark switching.

## Screenshots

### Main interface

![Cast Model Merger English interface](images/rust-native/main-window-en.png)

### Magazine filling (v2.2.1)

The following feature screenshots show the Chinese interface. The application also supports English, French, Russian and Spanish.

![Magazine bone detection and optional ammunition slots](images/rust-native/ammo-fill-zh.png)

### Model preview

All preview entry points share a full-window layout: a compact name row, model and perspective grid, lower-left camera tools, and bottom Viewport / Model info tabs. The existing macOS-inspired styling, light/dark themes and neutral model shading are preserved.

| Light theme | Dark theme |
| --- | --- |
| ![Light-theme model preview](images/rust-native/model-preview-zh.png) | ![Dark-theme model preview](images/rust-native/model-preview-dark-zh.png) |

## Download

Following the full Rust migration, [Releases](https://github.com/ez4cywa/ModelMergerGUI/releases/latest) provide only the native Windows x64 portable edition.

| Edition | Download | Runtime requirements |
| --- | --- | --- |
| Rust-native portable | [CastModelMerger-win-x64.zip](https://github.com/ez4cywa/ModelMergerGUI/releases/latest/download/CastModelMerger-win-x64.zip) | 64-bit Windows; **no .NET, Rust or additional runtime installation required** |

### System requirements

- 64-bit Windows 10 or 11. Windows 11 x64 with actively supported graphics drivers is recommended.
- A graphics driver supporting Direct3D 12. Rendering uses `wgpu`; if the application cannot create a window, update your Intel, AMD or NVIDIA graphics driver.
- No .NET Desktop Runtime or Rust installation is needed. Rust 1.96 is a source-build requirement only.

Extract the entire ZIP before running `CastModelMerger.exe`. The merge engine, application state, settings, localized interface and model preview are compiled into one native Rust application.

## Features

- Create independent merge groups and expand or collapse each one.
- Use a visual 5 × 3 slot grid with a selected-part count for each group.
- Click **Add next** or an empty slot to select multiple `.cast` files in one file-picker operation, or drag files into a group.
- Fixed-width part cards truncate long filenames. Hover to see the full path without squeezing the settings pane.
- Each group remembers its most recently used input folder for subsequent selections.
- Remove or replace parts and manually select a root model for each group.
- Open an interactive 3D preview of any imported part or the final merged model. Rust parses all previews, including models using 32-bit face indices.
- GPU depth-buffered rendering supports drag-to-rotate, wheel zoom, keyboard controls and view reset. Large previews are sampled without modifying source files.
- Fill magazine ammunition bones with copies of a cartridge model, and optionally select other numbered ammunition bones such as `j_ammo_17`.
- Start or cancel individual groups, or merge all ready groups at once.
- Run up to two merges concurrently; additional groups are queued. Queued and running jobs can both be cancelled.
- Prevent two model groups from writing to the same output path. Conflicts stop processing before mesh merging.
- Preserve the upstream root-detection, skeleton-connection and model-repositioning behavior by default.
- Choose an output folder and filename. Standard merging asks before overwriting an existing file; magazine filling always requires a new filename.
- Background processing, horizontal percentage progress bars, stage updates, logs and cancellation keep the interface responsive.
- Write output to a temporary file and read it back for validation before creating the final file.
- A fully native Rust architecture handles CAST parsing, merging, scheduling, settings, five-language UI and previews in one process. See the [full migration record (Chinese)](full-rust-migration.md).
- Switch instantly between 中文, English, Français, Русский and Español. Existing state, logs and dialogs update with the selected language.
- A prominent light/dark button at the right of the menu bar switches the entire app and previews, and saves your choice. New installations follow the system theme; restoring defaults returns to system mode.
- Embedded MiSans for Chinese; Segoe UI for the other four interface languages.
- The compact **File / Settings / Help** menu bar replaces the repeated heading and introduction. Create groups under **File**; choose a language or save/restore settings under **Settings**. Existing shortcuts are unchanged.
- Use **File → Open model preview…** (**Ctrl+O**) to open a CAST file in a standalone 3D preview without adding it to a merge group. Rotate, zoom and reset the view without changing the source file.
- Open **Help → About** to view the version, visit the GitHub repository and downloads, report issues, copy the project link, and check credits and licenses. The dialog follows the current language and light/dark theme.
- Check GitHub updates manually in **About**, or opt into startup checks (off by default). Same-version package refreshes are detected; downloads require a click and the app is never replaced automatically.
- Copy a source magazine's local ammunition positions and rotations to spare magazines without placement bones. Spares are individually selected and unchecked by default; new ammunition follows its own magazine bones.
- Magazine output also supports Windows filesystems without hard links, while preserving the requirement to save to a new filename.
- Save the interface language, output folder, root-model mode and window position. Selected model paths are not persisted.

The original WPF and command-line projects remain in the source tree for compatibility reference but are no longer included in release packages. The Rust GUI accepts 2–15 `.cast` parts per merge group and produces read-back-validated `.cast` output.

Settings are stored at:

```text
%LocalAppData%\CastModelMerger\settings.json
```

For catchable startup or runtime failures, a Chinese/English message offers access to the diagnostics folder. Logs include version, source commit, process and GPU information, preview file paths and loading/closing events. Rust panics force a backtrace; Windows native exceptions record their code and address on a best-effort basis. Normal exits record `shutdown`. Logs are stored at:

```text
%LocalAppData%\CastModelMerger\logs\CastModelMerger.log
```

For troubleshooting, share the log files in this directory, including `CastModelMerger-concurrent.log` if present. If the directory is unavailable, logs fall back to `%TEMP%\CastModelMerger\logs`. Logs stay local, include model file paths, and are never uploaded automatically. Forced termination or power loss may prevent the final error from being written.

## Usage

1. Click **New model group** to add a task. Collapse groups you do not need to view.
2. Click **Add next** or an empty slot in the target group and select one or more `.cast` files. Files fill the remaining slots in the order returned by the file picker.
3. Add 2–15 parts, or drag multiple files into the group. Files beyond the 15-part limit are not added, and a capacity notice is shown.
4. Click a part's **Preview** button to inspect it. Keep automatic root detection, or select manual root mode and use the part's set-as-root action.
5. Choose the group's output folder. Leave the output filename blank to use the root model's name.
6. Start the group or merge all ready groups using the bottom action bar. After a successful merge, preview the result from the group's status pane.

Choose a language in the **Settings** menu. Click **Save settings** to keep that choice for the next launch. On first launch, the application follows Windows when its language is one of the five supported languages; otherwise it defaults to Chinese.

The Chinese interface embeds Xiaomi MiSans. MiSans is not covered by this project's MIT license; its use and distribution follow Xiaomi's font license. See [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md) and the official license linked there.

The application uses a macOS-inspired layout, system light/dark themes and compact 36 px path/filename inputs. Main-window shortcuts: `Ctrl+N` creates a group, `Ctrl+S` saves settings, and `Ctrl+Enter` starts all ready groups.

## Fill ammunition at magazine bones

Each magazine appears once: its checkbox selects a fill target, and **Copy source** selects the layout for spare magazines. Hover over the magazine name to inspect bone details; the detail text is only generated when shown.

1. Click **Fill magazine** in any group's parts area. The tool prefers that group's merged output; you can also select a weapon or magazine CAST directly in the dialog.
2. Click the **Ammunition model** field and choose a cartridge CAST containing a single `tag_ammo` bone.
3. Wait for bone detection, then select the magazine groups to fill. Each group shows its bone count and occupied count. Only the first group is selected by default. Under **Other ammunition bones (optional)**, select individual locations such as `j_ammo_17`. You may deselect every magazine group and fill only these individual locations. Alternate animation magazines may overlap in the reference pose, so select them as needed.
4. Click **Save as** to choose an output file. The default is a `_filled.cast` file beside the weapon. Use a new filename; the original model is never overwritten.
5. Click **Fill magazine**. One cartridge instance is positioned, rotated and rigidly bound to each empty selected bone. Bones with existing mesh weights are skipped. After completion, click **Preview filled model**.

Detection recognizes `j_ammo_<digits>` and `tag_ammo_<digits>` beneath `j_mag`, `j_mag<digits>` or `tag_clip`. Counts come from the model's bones, not an assumed real-world weapon capacity. Numbered ammunition bones outside those subtrees are unchecked by default but can be selected individually.

Only unit-scale rigid skeletons are supported. Multi-bone cartridge sources, model-level transforms and non-unit bone scaling produce an error. One operation is limited to 512 slots and five million added vertices. See the [sample bone research (Chinese)](ammo-bone-research.md).

## Preview models

### Preview a part

1. Click **Add next** or an empty slot and choose one or more `.cast` files.
2. Each imported part displays its filename and a **Preview** button.
3. Click **Preview** to open an independent 3D window.
4. Open additional previews to compare parts side by side.

### Preview a merged model

1. Add 2–15 valid parts and check the output folder, choosing a different folder if needed.
2. Start the group and wait for a successful merge.
3. Click the merged-model preview button in the group's status pane. It becomes available only after a successful merge while the output file still exists.
4. If you move or delete the output file, merge again before previewing it.

### Preview controls

Drop one or more `.cast` files onto the dedicated **Model preview area** at the top of the workspace to open separate previews, or click it to select multiple files. This area ignores non-CAST files and leaves groups and settings unchanged. Dropping onto a group or part slot appends parts to that group; other areas place each batch in a separate group, reusing an idle empty group or creating one. You do not need to merge the previous batch first. The 15-part limit per group still applies.

| Action | Mouse or keyboard |
| --- | --- |
| Free rotation | Hold the left mouse button and drag |
| Step rotation | Use the rotate-left/right buttons or arrow keys |
| Zoom | Mouse wheel, zoom buttons, or `+` / `-` |
| Reset view | Reset-view button or `R` |
| Show or hide ground grid | Grid button or `G` |
| Toggle material preview | Checkerboard "Materials" button (off by default) |
| Inspect geometry and dimensions | Model info tab at the bottom |
| Close preview | Close button or `Esc` |

Previewing is read-only: it does not change parts, merge plans or output files. Geometry is prepared once in the background; the GPU handles rotation, zoom, shading and occlusion. A preview displays at most 250,000 triangles and shows a notice when sampled. Merging still uses the full model data.

### Material preview (optional)

By default the preview renders models in a neutral flat color. Click the checkerboard "Materials" button in the lower-left toolbar to switch to textured rendering:

- Textures come from the paths referenced by the material slots inside the `.cast` file (for example `_images/.../*.png` produced by exporters). Relative paths resolve against the cast file's folder; absolute paths are used as-is.
- Color textures are sampled as sRGB, normal/mask data as linear. Normal slots are decoded with the COD packed NOG convention (G/A jointly encode the tangent-space normal, R is a gloss candidate), based on the material reverse-engineering research in [ez4cywa-cod-blender-shaders](https://github.com/ez4cywa/ez4cywa-cod-blender-shaders).
- Textures decode on a background thread without blocking the UI; the status bar shows "Materials loaded/total". Missing or unreadable files fall back to a flat color per material and the status bar reports "Missing textures".
- Merged `.cast` outputs keep the texture paths of the input parts, so merged models support material previews too. Keep the output next to the exported asset folder so relative paths keep resolving.
- Material preview is equally read-only and never modifies source files or merge results.

#### Profile-based shading (aligned with the shader project)

The preview shader classifies each material by name and file stem and applies the control values from the shader project's `profiles.json`, so different material kinds render distinctly:

| Profile | Look | Matched by |
| --- | --- | --- |
| weapon | Metalness 1.0, albedo alpha as metal candidate | name contains `wpn_`/`_vm_`/`attachment`, or file stem contains `wpn_`/`vm_` |
| glass | Thin-wall transmission (IOR 1.46), fresnel highlights, blended transparency | name contains `glass`/`lens` |
| skin | SSS 0.22 wrap diffuse + light coat, IOR 1.4 | name contains `skin` |
| hair | Opacity cutout + sheen rim, normal strength 0.4 | name contains `hair` |
| eye / cornea / tearline | Strong clear-coat; cornea transmits and unplugs base color (white) | name contains `eye`/`iris`/`cornea`/`tear` |
| oral | Light SSS + coat | name contains `oral`/`teeth`/`gum` |
| cloth | Sheen 0.15 + roughness offset | name contains `cloth`, or file stem contains `_body_mp_`/`_head_mp_` |
| generic | Dielectric default | everything else |

Rules are first-match-wins. The research project's `_mat_info` techset rules do not apply to generic casts, so character sub-profiles use generic name hints instead. Materials classified as transmissive (glass/cornea) render in a separate alpha-blended pass after all opaque geometry.

## Build and test

Building the Rust GUI requires Rust 1.96 or a compatible newer stable toolchain. Release users do not need Rust or .NET.

MiSans permits embedding in an application but not redistributing the font as a standalone resource, so `.ttf` files are not committed to this repository. Before your first source build, read the [official MiSans license](https://hyperos.mi.com/font/en/download/), then run the following if you accept it:

```powershell
.\scripts\Install-MiSans.ps1 -AcceptLicense
```

The script downloads and verifies Xiaomi's official font package and extracts only the Medium weight used by the application. Chinese heading hierarchy relies on size, color and spacing rather than synthetic bold. The local font file is ignored by Git and embedded in the release executable. Official releases already include the embedded font; end users do not need to run this script.

```powershell
cd .\rust
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p model-merger-gui --bin CastModelMerger
```

The CAST decoder enforces budgets for nodes, properties, numeric values and text. Windows CI checks formatting, Clippy, tests, release builds and icon resources, with scheduled weekly decoder fuzzing. After installing `cargo-fuzz`, you can also run the following from the `rust` directory:

```powershell
cargo fuzz --fuzz-dir .\fuzz run decode
```

From the repository root, create the native Windows x64 portable package and SHA-256 checksum file:

```powershell
.\scripts\Publish-RustNative.ps1
```

## Project structure

```text
rust/crates/cast-codec              Strictly bounded CAST encoding/decoding
rust/crates/model-merger-engine     Merging, ammunition placement, validation, safe output, preview sampling
rust/crates/model-merger-app-core   Workspace, settings, five-language catalogs, two-worker scheduling
rust/crates/model-merger-gui        Native eframe/egui/wgpu desktop interface
rust/fuzz                          CAST decoder fuzzing entry point
src/ and tests/                    Legacy WPF compatibility references and fixtures
```

The migration-era `model-merger-worker` source is retained as a historical protocol reference, but is excluded from the default Rust workspace and all production build, test and release paths.

## Credits and license

The original ModelMerger was developed by Philip / Scobalula, with Cast support added by echo000. This project retains the original attribution and uses the [MIT License](../LICENSE).
