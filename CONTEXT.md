# Domain context

## Model merge plan

A model merge plan is the complete editable definition of one merge group: selected model parts, root-model choice, output choice, readiness, and creation of an executable merge request. The plan owns these invariants; desktop and command-line entry adapters do not reproduce them.

## Scheduled merge task

A scheduled merge task is one submitted merge request across its queued, running, completed, failed, or cancelled lifecycle. The scheduler owns concurrency and output-path claims.

## Merge engine

The merge engine combines loaded model parts using one root-selection and geometry-remapping implementation. GUI and CLI are entry adapters; Cast and SEModel are format adapters.

## Language catalog

The language catalog maps structured application meaning to user-facing text for one culture. Chinese and English are two real language adapters at the same presentation seam.
