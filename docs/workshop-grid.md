# Workshop Grid interface

Threadmark remains a local desktop code-analysis tool. Its primary work area is
repository files / relationship map / evidence inspector; Map, Impact, Changes,
Tests and Diagnostics keep their existing names and engine contracts.

## Design system

`apps/desktop/src/style.css` centralizes canvas/surface/text/border/accent/status,
spacing, radius and typography tokens. Light mode uses warm neutral surfaces;
dark mode uses charcoal surfaces with the same hierarchy. Forest green marks
selection, focus and primary actions. System fonts keep the app offline.
Controls use 3–5 px radii, borders and compact heights. Technical values use
monospace; ordinary controls and prose use the system UI font. Reduced-motion
preferences disable control transitions. No new dependencies were introduced.

Shared PageHeading, SectionHeading and EmptyState primitives provide ruled
headers and plain empty states. Existing buttons, inputs, selects, tables,
disclosure evidence lists and context menus share semantic CSS primitives.

## Screens and workflows

- Welcome: practical folder action, metadata rail and dated recent-repository rows.
- Shell: compact context/search bar, quiet tabs, source totals and version/status rail.
- Map: unadorned canvas, selected-node contrast, navigation and path toolbars;
  viewport-aware fit, Actual size, and a full-name neighborhood list.
- Inspector: grouped actions, readable source, relationship evidence and ruled sections.
- Impact / Tests: compact technical rows with hop counts and existing evidence paths.
- Changes: baseline form, source-file rows and a symbol-change table; removed symbols
  retain baseline evidence and are explicitly marked as baseline-only.
- Diagnostics: code/path column beside the explanation, with an explicit loading state.
- Settings: labelled nonmodal dialog, theme/editor/storage sections, Close and Escape;
  opening focuses the theme control and closing restores the Settings trigger.
- Narrow windows: the map/work area takes the available width. Files and Inspector
  controls open mutually exclusive drawers; choosing a file closes the Files drawer.

The former dotted map background, floating-panel shadows, duplicate node-name
styles and scattered color values were removed. No cards, charts, decorative
icons, network assets or invented metrics were added.

## Preserved behavior

Backend analysis, storage, indexing, watching, Git comparisons, export, editor
configuration, theme persistence and CLI behavior are unchanged. Search,
Ctrl+K/Enter, history, pan/zoom, paths, Shift+F10 context actions, evidence,
related tests, diagnostics, cancellation and error reporting remain available.

## Verification

2026-10-07–08: 17 frontend tests and 38 Rust tests passed. Frontend lint/typecheck,
production frontend build and native Tauri/NSIS build passed. Added regression
checks exercise Settings focus restoration, keyboard search navigation and the
Git table's retained removed-symbol evidence. Existing tests exercise watching,
impact, search/evidence/history, notification/persistence errors and cancellation.
Three map checks additionally cover a 24-node neighborhood, full names and
keyboard node/context/evidence actions, fit/physical-size restoration, and
viewport resizing/observer cleanup.

Native review uses the actual forecast-service fixture and its local Git baseline,
not replacement mock data. Screenshots are in `docs/screenshots/workshop-grid`.
The release review covers all major views and light/dark themes. A separate native
development configuration exercises 800-pixel windows without changing shipping
window dimensions. Selecting a file closes the Files drawer and loads the map;
Inspector opens with the selected source. The development console showed no
application exceptions. Four browser advisories about unnamed map fields were
resolved by naming the controls (analysis forms were named consistently too).
The final redesigned unsigned installer passed upgrade/uninstall/reinstall checks
with the database, theme and repository preserved. The installed desktop launched
with the new interface. Review logs and checksummed installer/source/CLI archives
are under ignored `artifacts/workshop-grid`.

The 2026-10-08 map follow-up was reviewed in the installed Windows candidate
using an indexed one-file fixture with 24 functions. The bounded neighborhood
shows 24 nodes and its limit marker. Fit selection, Actual size (100%), List view
and navigation from a full-name row to source/relationship evidence were checked
in WebView2. The overview hint keeps a stable footer so fitting does not change
the available map height. This is a usability review, not a frame-time benchmark.

## Remaining UI debt

Graph overview scales intentionally shrink the diagram; Actual size restores
readable glyphs and List view displays full names, kinds and source lines.
Long map labels use middle ellipses instead of compressed glyphs; SVG titles,
accessible names and the inspector retain the complete identifiers.
Pane resizing uses the existing
native CSS resize affordance. Maximum-size graph frame profiling and multi-hour
WebView sessions remain outside this redesign validation. Public Windows signing
still requires a real certificate; current candidates are unsigned.
