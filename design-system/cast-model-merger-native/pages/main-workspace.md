# Main workspace override

The generated landing-page pattern is not applicable to this desktop productivity workspace. Keep its accessible color tokens and restrained motion, then apply these native-tool overrides.

- Use a compact top command bar, scrollable collapsible group cards, and a persistent bottom status/action bar.
- Default viewport: 1180 × 860 logical pixels; minimum: 900 × 680. Reflow the slot grid from five to three columns before allowing horizontal scrolling.
- Chinese text uses only MiSans Medium. Other languages prefer the installed Segoe UI face, with MiSans Medium as fallback. Body text is 15 px; helper text is at least 13 px; section headings are 17–18 px.
- Primary text `#0F172A`, secondary text `#475569`, background `#F8FAFC`, panel `#FFFFFF`, border `#CBD5E1`, primary/focus `#2563EB`, destructive `#DC2626`.
- Interactive targets are at least 44 px high. Slot cards are at least 136 × 108 px. Keep 8 px between controls and 16 px panel padding.
- Every field has a persistent visible label. Put validation and task errors next to the affected group; never rely on red color alone.
- Preserve visible keyboard focus. Logical tab order follows top commands → group summary → slots → group settings → group actions → bottom actions.
- Motion is limited to native focus/hover feedback and collapsible panels. Do not animate layout size or use decorative reveals.
- Filled slots expose Preview, Replace, Set as root, and Remove through labeled controls or a keyboard-accessible context menu. Empty slots use an explicit Add part label.
