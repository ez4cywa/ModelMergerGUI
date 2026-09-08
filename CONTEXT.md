# Domain context

## Model merge plan

A model merge plan is the complete editable definition of one merge group: selected model parts, root-model choice, output choice, readiness, and creation of an executable merge request. The plan owns these invariants; desktop and command-line entry adapters do not reproduce them.

## Scheduled merge task

A scheduled merge task is one submitted merge request across its queued, running, completed, failed, or cancelled lifecycle. The scheduler owns concurrency and output-path claims.

## Merge engine

The merge engine combines loaded model parts using one root-selection and geometry-remapping implementation. GUI and CLI are entry adapters; Cast and SEModel are format adapters.

## Language catalog

The language catalog maps structured application meaning to user-facing text for one culture. Simplified Chinese, English, French, Russian, and Spanish are real language adapters at the same presentation seam. The native GUI selects embedded MiSans for Chinese and installed Segoe UI for the other supported languages.

## Native application core

The native application core owns editable merge groups, schema-compatible settings, five-language text keys, scheduled-task state, cancellation and output-path claims. Native GUI code renders this state and forwards user intent; it does not repeat these invariants.

## Model preview

A model preview is a bounded, read-only projection of Cast mesh geometry for interactive display. Core owns loading, statistics, validation and triangle sampling; the native WGPU module owns camera transforms, lighting and depth-tested rendering. Preview sampling never changes the selected part or merged output.

## Ammunition placement

The ammunition engine owns skeleton analysis, rigid placement, optional spare-magazine layout replication, and no-overwrite output publication. Spare layouts are transformed relative to the chosen source magazine and receive new bones under the target magazine. Magazine ancestors such as `tag_clip` are not themselves spare targets. On Windows filesystems without hard-link support, a same-directory move without replacement publishes the verified temporary file.

## Release checks

GitHub release checks are explicit or opt-in at startup; `checkUpdatesOnStartup` defaults to false in existing and new settings. Checks run off the UI thread, compare release versions and source commit identities for same-version refreshes, and only expose this repository's portable ZIP download. Checking never downloads or installs an update automatically.
