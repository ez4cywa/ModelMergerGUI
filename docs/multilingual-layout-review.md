# Multilingual layout review — v2.2.1 refresh

Checked on Windows with the application's actual egui renderer, retaining the existing macOS-inspired design and font sizes. The same-version rebuild replaces the v2.2.1 release package.

| Surface | Languages | Logical viewport | Checks |
| --- | --- | --- | --- |
| Main workspace, empty and imported part | English, French, Russian, Spanish | 1180 × 860 and 900 × 680 | Toolbar text, filename truncation, part controls, settings alignment, bottom actions |
| Main workspace, dark theme | English, French, Russian, Spanish | 900 × 680 | Text legibility and small-window layout |
| Magazine filling | English, French, Russian, Spanish | 900 × 680 | Long explanations, optional bones, scrolling, persistent fill action |
| Model preview | English, French, Russian, Spanish | 640 × 480 | Paired controls, wrapping, model name and metadata, actual GPU rendering |

Fixes use measured text widths, explicit spacing for the group chevron, compact full-width part preview controls, top-aligned settings, localized Browse-button sizing, and an independently scrolling ammunition form. Font sizes were not reduced to fit translations. Longer model grids remain vertically scrollable at the minimum window size.

Automated tests cover all five languages: command-bar text bounds and collisions at 900/1180/1440 px widths; visible magazine-fill actions with 128 bone names; all six preview commands at 640 × 480. Native file pickers are owned by Windows, not custom layouts. This audit does not claim exhaustive DPI/monitor or screen-reader testing.

Final main-window captures: [English](images/rust-native/main-window-en.png), [French](images/rust-native/main-window-fr.png), [Russian](images/rust-native/main-window-ru.png), [Spanish](images/rust-native/main-window-es.png).
