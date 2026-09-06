# Cast Model Merger GUI specification

## Product scope

- Build a Windows x64 native desktop GUI from `echo000/ModelMerger` using Rust, eframe/egui and wgpu.
- The GUI accepts Cast (`.cast`) model parts only.
- A merge contains at least 2 and at most 15 unique, existing files.
- Keep one multilingual executable with live Simplified Chinese, English, French, Russian, and Spanish switching.
- Keep the former WPF/console implementation only as migration-test material; do not include it in the native release.
- Preserve the upstream MIT licence and author attribution.

## Part selection

- The workspace can contain multiple independent merge groups.
- Each group can be expanded or collapsed without losing its selections or progress.
- A group can merge independently; “merge all ready groups” starts groups concurrently with a safe maximum of two active merges.
- The scheduler owns queued, running, succeeded, failed, and cancelled task states; queued and running tasks can be cancelled.
- Concurrent groups cannot claim the same resolved output path, and conflicts stop before mesh merge work.
- Show 15 numbered slots in a 5 by 3 visual layout and an `n / 15` counter.
- An empty slot and “Add next part” both open a multi-selection file dialog and append the returned files to the remaining slots in order.
- Within each group, subsequent part dialogs start in the directory of the most recently accepted part.
- Drag-and-drop can add multiple parts up to the remaining capacity; the group under the pointer shows a text-and-border drop target before release.
- A filled slot uses a fixed card width, truncates long file names with an ellipsis, exposes the full path on hover, and can be removed or replaced without widening the workspace or squeezing the settings column.
- A filled slot can open an interactive 3D preview without changing the selected file or merge plan.
- Reject duplicates, missing files, non-Cast files, and additions beyond slot 15 without disturbing accepted slots.
- Disable merging until at least two valid parts are selected.

## Merge and output

- Preserve the upstream automatic root-model and bone-connection behaviour by default.
- Allow a user to mark one selected part as the manual root.
- Run loading and merging away from the UI thread and report stage progress through an always-visible horizontal percentage progress bar plus localized status text.
- Allow cancellation at safe processing points.
- Let the user choose the output folder and output file name.
- Default to a `Merged Models` folder next to the first selected part and the root model name.
- Confirm before overwriting an existing output.
- Save to a temporary file, verify that it can be loaded, then promote it to the final `.cast` file.
- Remove temporary output after failure or cancellation.
- Show actionable warnings for disconnected skeletons and actionable errors for invalid Cast files. Selection and scheduling errors remain attached to the affected group instead of moving to a global banner.
- Each group has independent inputs, root selection, output, progress, cancellation, log, and result.
- A completed group can preview its merged Cast output directly from the group status card.
- Preview loading and geometry preparation run away from the UI thread. Large models are sampled to at most 250,000 triangles for display only; source and output files remain unchanged.
- Preview vertex transformation, lighting and occlusion use the WGPU pipeline and a depth buffer; interactive frames do not rebuild or depth-sort the full triangle list on the CPU.
- Every preview uses the in-process Rust CAST loader, including small files with 32-bit face indices.
- Preview interaction supports mouse drag rotation, wheel zoom, visible rotate/zoom/reset controls, keyboard alternatives, and Escape/Close dismissal.
- Core exposes structured progress, warning, validation, read-error, and output-conflict semantics so each presentation adapter can localize them.
- Run the Rust merge engine in-process behind the native two-task scheduler; no worker executable or NDJSON adapter is part of the release.

## Saved settings

- Provide Save settings and Restore defaults actions.
- Store settings under `%LocalAppData%\CastModelMerger\settings.json` rather than the registry.
- Save the selected interface language, preferred output folder, whether to remember that folder, automatic/manual root mode, and the last valid window bounds.
- Do not save selected model paths.
- Fall back to defaults if the settings file is missing, corrupt, or points to an unusable directory.

## Localization

- The user can switch between Simplified Chinese, English, French, Russian, and Spanish without restarting.
- Static UI, existing group status, existing run logs, file dialogs, confirmations, validation errors, warnings, and completion messages update to the selected language.
- On first launch, use the Windows UI language when it is one of the five supported languages; otherwise default to Simplified Chinese.
- Language resources have matching keys and format placeholders.
- The Simplified Chinese interface uses the build-time embedded MiSans Medium face and visibly acknowledges MiSans; other languages use installed Segoe UI with MiSans as the CJK fallback.
- Embed only the original MiSans Medium font file; Chinese text uses that weight consistently, while size, color, and spacing carry the heading hierarchy without duplicate full CJK font files.
- Main body text uses a 15 px medium-weight baseline. Helper and caption text stays at 12–13 px minimum with a darker high-contrast foreground, including disabled actions.

## Distribution and acceptance

- Publish only the Rust-native `win-x64` ZIP containing `CastModelMerger.exe`, README, MIT license and third-party notices.
- The release does not require the Microsoft .NET Desktop Runtime or a Rust runtime; Rust 1.96 or newer is a source-build prerequisite only.
- Build and tests must pass.
- Windows CI must enforce formatting, warning-free Clippy, workspace tests, native release compilation and embedded/runtime icon checks. Tag releases must match the Cargo workspace version, rebuild their assets from the tagged clean tree and publish the ZIP together with its SHA-256 file.
- Startup panics and renderer initialization failures must be written under `%LocalAppData%\CastModelMerger\logs` and shown through a bilingual recovery dialog because the release binary has no console window.
- CAST decoding must enforce configurable aggregate resource budgets and be exercised by a scheduled fuzz target in addition to deterministic malformed-input tests.
- Tests cover the merge-plan seam, scheduler lifecycle/concurrency/cancellation/output conflicts, settings round-trip, structured merge semantics, five-language completeness and live rerendering, AccessKit output, responsive native rendering, preview cancellation, malformed inputs, output cleanup, and real 2/8/15-part Cast merges that are readable afterward.
