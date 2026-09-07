# Main workspace override

This page follows the macOS-inspired native utility contract in `MASTER.md`.

- Use a compact top command bar, scrollable collapsible group cards, and a persistent bottom status/action bar.
- Default viewport: 1180 × 860 logical pixels; minimum: 900 × 680. Reflow the slot grid from five to three columns before allowing horizontal scrolling.
- Chinese text uses only MiSans Medium. Other languages prefer the installed Segoe UI face, with MiSans Medium as fallback. Body and control text is 15 px; helper text is at least 13 px; section headings are 17 px.
- Use the paired semantic light/dark palettes from `MASTER.md`. Follow the operating system theme instead of adding an in-app theme switch.
- Standard command controls are 40 px high; path/name fields and their adjacent Browse control are 36 px high. Keep at least 8 px between controls. Slot cards are at least 136 × 108 px. Keep 16 px group padding and 24 px workspace margins.
- Give every slot card an explicit grid-derived width and a vertical internal layout. Truncate long file names with an ellipsis and expose the complete path in a tooltip; slot content must never widen the left pane or reduce the reserved 300 px settings pane.
- “Add next” and empty-slot actions use the same multi-select Cast picker. Accepted files fill the remaining slots in returned order and the 15-part capacity remains visible.
- Group status uses a full-width 12 px horizontal progress track with a separate visible percentage and localized state label. The track remains visible at 0%, and the progress widget keeps its accessibility role and numeric value.
- Every field has a persistent visible label. Put validation and task errors next to the affected group; never rely on red color alone.
- Preserve visible keyboard focus. Logical tab order follows top commands → group summary → slots → group settings → group actions → bottom actions.
- Provide `Ctrl/⌘N`, `Ctrl/⌘S`, and `Ctrl/⌘Enter` as accelerators for new group, save settings, and merge all ready groups.
- Motion is limited to native focus/hover feedback and collapsible panels. Do not animate layout size or use decorative reveals.
- Filled slots expose Preview, Replace, Set as root, and Remove through labeled controls or a keyboard-accessible context menu. Empty slots use an explicit Add part label.
