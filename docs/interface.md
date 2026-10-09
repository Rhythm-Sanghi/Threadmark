# Interface

The workspace has a repository tree, a relationship map and an evidence inspector.
Map, Impact, Changes, Tests and Diagnostics share the selected repository and symbol.

## Navigation

- Ctrl+K focuses search. Enter opens the selected result.
- Back and Forward revisit selected nodes.
- Select a relationship to inspect its source span, confidence and evidence.
- Right-click a node, or use Shift+F10, for impact, related tests and path actions.
- Drag the map to pan; scroll or use the toolbar to zoom.
- Fit selection shows the whole neighborhood. Actual size restores 100% labels.
- List view shows full names, kinds and source lines for the current neighborhood.

A map contains at most 24 nodes. The limit marker means more relationships exist.
Long labels are shortened on the map; full names remain in titles and the inspector.
At narrow widths, Files and Inspector open separate drawers.

## Appearance

Settings supports light, dark and system themes. Settings are shared with the CLI.
Escape closes settings and returns focus to its button. Reduced-motion preferences
are respected.

The stylesheet defines shared color, spacing, type and control tokens. Panels use
borders and aligned rows; paths and source use monospace. PageHeading,
SectionHeading and EmptyState provide the shared section structure.

[Screenshots](screenshots/README.md) show the current views. Native checks and
remaining platform limits are documented in [release notes](release.md).
