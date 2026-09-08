# Cast Model Merger native design contract

**Product:** a focused desktop utility for assembling two to fifteen Cast model parts in one or more independent groups.

**Design direction:** calm macOS-inspired utility interface implemented with native egui controls. At the user's explicit request (2026-09-08), the main window uses an application-drawn macOS-style title bar with red/yellow/green window controls and a centered title. Preserve native Windows drag, resize, minimize, maximize/restore and close behavior underneath; do not replace other platform interactions.

## Product character

- Quiet, precise, compact, and work-oriented.
- The model groups and their state are the visual focus. Settings support the task and remain visually subordinate.
- Use progressive disclosure: collapsed groups show identity, part count, readiness, and actions; expanded groups reveal slots and settings.
- No ornamental dashboard cards, hero areas, gradients, floating pills, emoji icons, decorative blur, or layout-shifting hover effects.

## Semantic color system

| Role | Light | Dark | Use |
|---|---|---|---|
| Background | `#F5F5F7` | `#1C1C1E` | Workspace |
| Panel | `#FFFFFF` | `#242426` | Model group cards and editable fields |
| Toolbar | `#F7F7F9` | `#2A2A2C` | Top and bottom command areas |
| Sidebar | `#F6F6F8` | `#2C2C2E` | Group settings |
| Surface | `#FAFAFB` | `#323234` | Empty slot cards |
| Foreground | `#1D1D1F` | `#F5F5F7` | Primary text |
| Secondary | `#545458` | `#BEBEC4` | Helper text and metadata |
| Border | `#D1D1D6` | `#48484A` | Separators and card outlines |
| Primary | `#0066CC` | `#0A72E8` | Primary actions, focus, progress |
| Destructive | `#C62828` | `#FF6961` | Removal and errors |

All normal text/background and button text/fill pairs must meet WCAG AA contrast in both themes. State must never be conveyed by color alone.

## Typography

- Chinese: bundled MiSans Medium for all UI text.
- Other languages: installed Segoe UI, with MiSans Medium as fallback.
- Type scale: 22 px app title, 17 px section/group titles, 15 px body and controls, 13 px helper/status text, 12 px slot indexes.
- Keep weight consistent through the selected font file. Create hierarchy with size, spacing, and color rather than mixing arbitrary font weights.

## Geometry and spacing

- Use an 8 px spacing rhythm; use 4 px only inside compact badges.
- Window/card/panel radii: 12/12/10 px. Slot and notice radius: 9 px. Controls and fields: 7 px.
- Standard command controls are 40 px tall. Single-line path/name fields and their adjacent Browse control are 36 px tall so the frame fits the 15 px text without excess vertical space. Preserve a clear hit region and at least 8 px between adjacent controls.
- Card shadow is subtle: 2 px vertical offset, 10 px blur, very low opacity. Menus and dialogs may use stronger system-provided elevation.
- Motion is limited to the native 180 ms hover/focus feedback and collapsible group state. Never animate layout position or size.

## Interaction contract

- The top toolbar contains language and infrequent workspace commands. The bottom action area contains persistent batch state and the single primary action.
- Drag-and-drop is optional acceleration. Every drag action has the labeled Add control as a keyboard and pointer alternative.
- File names truncate inside fixed slot widths and expose the full path in a tooltip.
- Keyboard order follows the visible reading order. Preserve visible native focus feedback.
- Shortcuts: `Ctrl/⌘N` new group, `Ctrl/⌘S` save settings, `Ctrl/⌘Enter` merge all ready groups.
- Loading, ready, disabled, error, missing-file, empty, completed, drag-over, and overwrite-confirmation states must remain explicit and readable.

## Finish gate

Before shipping, inspect a real render at default and minimum size in both light and dark modes. Reject the build for clipped labels, horizontal overflow, hidden focus, low contrast, overlong filenames widening the layout, competing primary actions, or missing task feedback.
