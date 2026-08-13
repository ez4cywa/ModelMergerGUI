# Cast Model Merger GUI specification

## Product scope

- Build a Windows desktop GUI from `echo000/ModelMerger` using C# and .NET 8.
- The GUI accepts Cast (`.cast`) model parts only.
- A merge contains at least 2 and at most 16 unique, existing files.
- Keep the upstream console project available as a regression reference.
- Preserve the upstream MIT licence and author attribution.

## Part selection

- The workspace can contain multiple independent merge groups.
- Each group can be expanded or collapsed without losing its selections or progress.
- A group can merge independently; “merge all ready groups” starts groups concurrently with a safe maximum of two active merges.
- Show 16 numbered slots in a 4 by 4 visual layout and an `n / 16` counter.
- An empty slot can add one part with a single-selection file dialog.
- “Add next part” adds one file to the first empty slot.
- Within each group, subsequent part dialogs start in the directory of the most recently accepted part.
- Drag-and-drop can add multiple parts up to the remaining capacity.
- A filled slot shows its file name and validation state and can be removed or replaced.
- Reject duplicates, missing files, non-Cast files, and additions beyond slot 16 without disturbing accepted slots.
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

## Saved settings

- Provide Save settings and Restore defaults actions.
- Store settings under `%LocalAppData%\CastModelMerger\settings.json` rather than the registry.
- Save the preferred output folder, whether to remember that folder, automatic/manual root mode, and the last valid window bounds.
- Do not save selected model paths.
- Fall back to defaults if the settings file is missing, corrupt, or points to an unusable directory.

## Distribution and acceptance

- Produce an unpackaged, self-contained `win-x64` release.
- The app must start without a separately installed .NET runtime.
- Build and tests must pass.
- Tests cover 2/16 accepted parts; 0/1/17 rejected merge requests; duplicates; invalid extension; missing files; settings round-trip and corrupt-file fallback; cancellation cleanup; and a real synthetic Cast merge that is readable afterward.
