# Cast Model Merger GUI specification

## Product scope

- Build a Windows desktop GUI from `echo000/ModelMerger` using C# and .NET 8.
- The GUI accepts Cast (`.cast`) model parts only.
- A merge contains at least 2 and at most 15 unique, existing files.
- Keep one multilingual executable with live Simplified Chinese, English, French, Russian, and Spanish switching.
- Keep the upstream console entry point, but make it use the same Core merge engine as the GUI.
- Preserve command-line drag-and-drop support for one or more `.cast` / `.semodel` inputs; output remains Cast.
- Preserve the upstream MIT licence and author attribution.

## Part selection

- The workspace can contain multiple independent merge groups.
- Each group can be expanded or collapsed without losing its selections or progress.
- A group can merge independently; “merge all ready groups” starts groups concurrently with a safe maximum of two active merges.
- The scheduler owns queued, running, succeeded, failed, and cancelled task states; queued and running tasks can be cancelled.
- Concurrent groups cannot claim the same resolved output path, and conflicts stop before mesh merge work.
- Show 15 numbered slots in a 5 by 3 visual layout and an `n / 15` counter.
- An empty slot can add one part with a single-selection file dialog.
- “Add next part” adds one file to the first empty slot.
- Within each group, subsequent part dialogs start in the directory of the most recently accepted part.
- Drag-and-drop can add multiple parts up to the remaining capacity.
- A filled slot shows its file name and validation state and can be removed or replaced.
- A filled slot can open an interactive 3D preview without changing the selected file or merge plan.
- Reject duplicates, missing files, non-Cast files, and additions beyond slot 15 without disturbing accepted slots.
- Disable merging until at least two valid parts are selected.

## Merge and output

- Preserve the upstream automatic root-model and bone-connection behaviour by default.
- Allow a user to mark one selected part as the manual root.
- Run loading and merging away from the UI thread and report stage progress.
- Allow cancellation at safe processing points.
- Let the user choose the output folder and output file name.
- Default to a `Merged Models` folder next to the first selected part and the root model name.
- Confirm before overwriting an existing output.
- Save to a temporary file, verify that it can be loaded, then promote it to the final `.cast` file.
- Remove temporary output after failure or cancellation.
- Show actionable warnings for disconnected skeletons and actionable errors for invalid Cast files.
- Each group has independent inputs, root selection, output, progress, cancellation, log, and result.
- A completed group can preview its merged Cast output directly from the group status card.
- Preview loading and geometry preparation run away from the UI thread. Large models are sampled to a bounded triangle count for display only; source and output files remain unchanged.
- Preview interaction supports mouse drag rotation, wheel zoom, visible rotate/zoom/reset controls, keyboard alternatives, and Escape/Close dismissal.
- Core exposes structured progress, warning, validation, read-error, and output-conflict semantics so each presentation adapter can localize them.

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
- The Simplified Chinese interface uses embedded MiSans fonts and visibly acknowledges MiSans; other languages use Segoe UI.

## Distribution and acceptance

- Produce an unpackaged, self-contained `win-x64` release.
- The app must start without a separately installed .NET runtime.
- Build and tests must pass.
- Tests cover the merge-plan seam, scheduler lifecycle/concurrency/cancellation/output conflicts, shared Cast/SEModel engine, settings round-trip, structured merge semantics, five-language completeness and live rerendering, WPF rendering in all supported languages, cancellation cleanup, and a real synthetic Cast merge that is readable afterward.
