# Full Rust migration map

## Destination

Ship a Windows x64 Cast Model Merger whose production executable, application state, task orchestration, merge engine, preview loader, 3D renderer, settings, localization, and GUI are implemented in Rust. The final package must not require the .NET Desktop Runtime and must preserve the current five-language feature set and 2–15 part compatibility.

## Execution notes

- This map carries implementation as well as decisions because the user explicitly requested completion of the whole migration.
- The WPF application remains in the source tree only as a compatibility oracle. It is not included in the final Rust-only package and no WPF release script remains.
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
- Compatibility/performance corpus and the temporary WPF adapter used during migration.

### 2. Rust preview pipeline — complete

- Parse, validate, sample and bound preview geometry in Rust.
- Expose compact preview geometry to the native renderer without JSON float arrays.
- Route every production preview through the Rust codec and sampler.
- Gate on C#/Rust geometry hashes, 32-bit index models, cancellation and payload cleanup.

Acceptance closed with matching C#/Rust geometry hashes, a real sub-2 MiB model using 65,537 vertices and 32-bit face indices, and cancellation tests at the public preview seam. The final native GUI no longer uses the migration worker payload; loading and rendering are in-process.

### 3. Rust application core — complete

- Move merge-group state, validation, output planning and the two-task scheduler into Rust modules.
- Move settings schema, atomic persistence and five-language catalogs into Rust.
- Expose narrow interfaces usable by both the compatibility UI and native GUI tests.

Acceptance closed with public-seam tests for 15-slot group invariants, path-alias rejection and remembered input directories; schema-compatible settings replacement with interrupted-write recovery; 88 exhaustive interface and dynamic-status keys across all five languages; a hard two-task concurrency limit, bounded shutdown, queued cancellation, resolved output-path claims, structured engine errors, and a native two-part merge.

### 4. Native Rust GUI tracer — complete

- Create the eframe shell, MiSans/Segoe-compatible font configuration and five-language switching.
- Implement collapsible groups, the 5 × 3 slot grid, remembered input directory and output selection.
- Add screenshot and accessibility-tree tests before expanding behavior.

Acceptance closed with a real Windows wgpu window review at 1180 × 860, responsive 5/3/2-column layout tests, AccessKit trees in all five languages, embedded MiSans Medium, remembered dialogs and collapsible multi-group state.

### 5. Native execution and 3D preview — complete

- Connect group controls to the Rust scheduler and merge engine without a worker process.
- Implement wgpu model preview with rotate, zoom, reset, keyboard alternatives and triangle-limit messaging.
- Preserve multi-window preview and cancellation behavior.

Acceptance closed with direct in-process scheduling, overwrite confirmation, structured localized task errors, cancellable background preview loading, independent preview viewports, mouse/keyboard controls and bounded 75,000-triangle display sampling.

### 6. Cutover — complete

- Match every item in `docs/spec.md` against the native application.
- Build a single Rust-only Windows x64 package with license, MiSans notice and README.
- Remove the .NET runtime requirement from release documentation and stop packaging the WPF executable.

The only production package is now built by `scripts/Publish-RustNative.ps1`. WPF sources remain solely as compatibility evidence and are excluded from the ZIP.

### 7. Release acceptance — complete

- Run 2/8/15-part semantic corpora, malformed-input suites and output safety tests.
- Run five-language screenshot, keyboard, high-DPI and accessibility checks.
- Record merge and preview performance, binary size and peak-memory comparisons.
- Publish only after the Rust package passes all gates and the compatibility fallback is no longer needed.

Acceptance closed on Windows x64 with the complete workspace test suite and warning-free Clippy, including 2/8/15-part semantic merges, malformed/truncated CAST inputs, overwrite safety, cancellation cleanup, five-language catalogs and AccessKit trees. Real wgpu windows were reviewed at 1180 × 860 and the 900 × 680 minimum in Chinese, English and long-label French layouts; the preview was checked with mouse and keyboard alternatives. The README screenshots were recaptured from the accepted x64 executable on 2026-08-27.

The migration benchmark corpus measured Rust merge medians of 35/81/182 ms for 2/8/15 parts versus 349/1,384/2,472 ms for the former C# path. Preview medians were 43.5 ms for Rust versus 68.2 ms for C# on 20,000 triangles, while a 150,000-vertex 32-bit-index model loaded in 81.1 ms in Rust. These are local development-machine measurements, not universal performance promises.

The final stripped executable is 21,259,776 bytes and the release ZIP, including documentation screenshots, is 11,809,125 bytes. A single idle native main window used approximately 239 MiB working set and a loaded preview approximately 262 MiB on the acceptance machine; the former WPF main window used approximately 135 MiB. The native renderer therefore uses more idle memory, while merge and preview processing are substantially faster and no .NET runtime or worker process is required.
