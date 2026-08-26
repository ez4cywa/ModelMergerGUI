# Full Rust migration map

## Destination

Ship a Windows x64 Cast Model Merger whose production executable, application state, task orchestration, merge engine, preview loader, 3D renderer, settings, localization, and GUI are implemented in Rust. The final package must not require the .NET Desktop Runtime and must preserve the current five-language feature set and 2–15 part compatibility.

## Execution notes

- This map carries implementation as well as decisions because the user explicitly requested completion of the whole migration.
- The WPF application remains buildable as the compatibility oracle until the native Rust acceptance gates pass. It is not included in the final Rust-only package.
- Each phase uses public-seam golden tests and must remain releasable before the next phase starts.
- Initial production scope remains Windows x64. Cross-platform packaging is outside this migration.

## Decisions so far

- CAST decoding, merge semantics, safe output and cancellation live in the existing Rust workspace.
- The native GUI will use [eframe/egui](https://github.com/emilk/egui) with its wgpu renderer. The official framework exposes custom wgpu rendering and enables AccessKit integration for native accessibility.
- Native file and folder selection will use [rfd](https://github.com/PolyMeilex/rfd).
- MiSans Medium remains the only embedded Chinese face; hierarchy uses size, color and spacing so the package does not contain duplicate full CJK fonts.
- Settings remain JSON under `%LocalAppData%\CastModelMerger` and receive an explicit schema version.

## Migration phases

### 1. Rust merge engine — complete

- Strict bounds-checked CAST codec.
- Flat-buffer model representation and 2–15 part merge semantics.
- Safe temporary output, verification, overwrite handling and cancellation.
- Adaptive WPF adapter and compatibility/performance corpus.

### 2. Rust preview pipeline — complete

- Parse, validate, sample and bound preview geometry in Rust.
- Transfer compact geometry through a worker-owned binary payload.
- Route large previews to Rust while retaining small-file C# fallback.
- Gate on C#/Rust geometry hashes, 32-bit index models, cancellation and payload cleanup.

Acceptance closed with matching C#/Rust geometry hashes, a real sub-2 MiB model using 65,537 vertices and 32-bit face indices, and cleanup tests for acknowledgement, cancellation and input-pipe closure. Decode, payload write and payload read loops check cancellation at bounded intervals.

### 3. Rust application core — next

- Move merge-group state, validation, output planning and the two-task scheduler into Rust modules.
- Move settings schema, atomic persistence and five-language catalogs into Rust.
- Expose narrow interfaces usable by both the compatibility UI and native GUI tests.

### 4. Native Rust GUI tracer

- Create the eframe shell, MiSans/Segoe-compatible font configuration and five-language switching.
- Implement collapsible groups, the 5 × 3 slot grid, remembered input directory and output selection.
- Add screenshot and accessibility-tree tests before expanding behavior.

### 5. Native execution and 3D preview

- Connect group controls to the Rust scheduler and merge engine without a worker process.
- Implement wgpu model preview with rotate, zoom, reset, keyboard alternatives and triangle-limit messaging.
- Preserve multi-window preview and cancellation behavior.

### 6. Cutover

- Match every item in `docs/spec.md` against the native application.
- Build a single Rust-only Windows x64 package with license, MiSans notice and README.
- Remove the .NET runtime requirement from release documentation and stop packaging the WPF executable.

### 7. Release acceptance

- Run 2/8/15-part semantic corpora, malformed-input suites and output safety tests.
- Run five-language screenshot, keyboard, high-DPI and accessibility checks.
- Record merge and preview performance, binary size and peak-memory comparisons.
- Publish only after the Rust package passes all gates and the compatibility fallback is no longer needed.
