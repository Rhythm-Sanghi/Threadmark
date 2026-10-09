# Threadmark status

## Current stage

Stages 0–10 are implemented and locally validated for the early Windows 0.1.1
build. Release artifacts are produced locally; no public release is claimed.

## Complete

- 0: Rust core/database/CLI/Tauri workspace, React shell, GPL license, local storage,
  transactional migrations, settings, icon, lockfiles and Windows CI definition.
- 1: Bounded static JS/TS discovery and syntax indexing, ignores, cancellation,
  versioned parse caches, persisted snapshots and visible diagnostics.
- 2: Local imports, aliases, workspace packages, reexports, calls/types/references,
  confidence, evidence, ambiguity handling and circular import components.
- 3: Real-data search/tree/map, directed neighborhoods, history, paths, inspector,
  source preview, configured editor, pan/zoom and keyboard node actions.
- 4: Bounded reverse impact with direct/transitive/possible groups and evidence paths.
- 5: Local Git ref comparison, symbol/relationship changes, refreshed working tree,
  deleted-symbol baseline evidence and Windows line-ending normalization.
- 6: Static test discovery and detected related tests with inspectable relationships.
- 7: Literal Express routes, JSX component references and environment-key relationships.
- 8: Python syntax/import/call/class/method/test adapter without Python execution.
- 9: Native file watching, debounce, hash reuse, rename/delete handling and atomic save.
- 10: Production installer/CLI/source packaging, notices, native reviews, screenshots,
  measured performance, documented limits and security boundary review.

## Interface update

The Workshop Grid redesign uses a compact split workspace, shared light/dark
tokens, technical rows and Git change tables. Features and backend contracts are
preserved. Native reviews covered all major views and an 800-pixel work area with
Files/Inspector drawers. See [design and validation](docs/workshop-grid.md).

## Checks run

- 38 Rust tests passed: storage/migrations, CLI, indexing/resolution, ambiguity,
  impact/cycles, Git/removal/CRLF, tests, frameworks/configuration, Python, native
  watching, script non-execution, Unicode and Windows junction boundaries.
- 17 frontend interaction tests passed, including notification failures,
  keyboard context menus and full-size map/list navigation on 24-node slices.
  Lint, typecheck and production frontend build passed.
- Cargo formatting and all-target workspace Clippy with warnings denied passed.
- Release CLI and native Tauri/NSIS production builds passed.
- NSIS installed into an isolated current-user test folder. Installed CLI and
  desktop launched; CLI, GPL and third-party notices are present. Installer
  registration reports version 0.1.1 and the tested installation location.
- Upgraded the existing 0.1.0 installation to 0.1.1. Schema 2 migrated to 3
  with the selected repository and theme preserved. Automated current-user
  uninstall/reinstall checks passed; uninstall preserved the user database.
  Binary verification accounts for Tauri's exact NSIS bundle-marker patch.
- Cross-connection graph revision invalidation and repository selection isolation,
  unchanged parse-cache persistence, relative tsconfig inheritance and conservative
  suppression of calls through reassigned JS bindings have regression coverage.
- Installed desktop reviewed using indexed JS/TS/Python source and actual Git:
  first open, Ctrl+K search/Enter, map/edge evidence, impact, related tests,
  Changes, removed-call baseline evidence, diagnostics, fit/zoom and Shift+F10.
- Native watching refreshed the index after a source edit (one parsed/eight reused);
  original source restored and watching stopped. Configured Notepad opened the
  actual indexed file. VS Code launch was attempted, but its window did not appear
  on this host; VS Code positional navigation is not claimed as verified.
- Native metadata save dialog wrote JSON; snippets and free-form diagnostic
  messages were confirmed absent. Explicit CLI evidence export has test coverage.
- Real Threadmark frontend source indexed and queried through installed CLI using
  a separate data directory. Screenshots are in docs/screenshots.
- Generated Rust/npm license inventories and collected dependency/vendored notices.
  On 2026-10-07, cargo audit reported zero vulnerability advisories and two
  informational warnings: proc-macro-error 1.0.4 (unmaintained) and glib 0.18.5
  (unsound). Neither package is in the Windows target dependency graph. npm audit
  reported zero vulnerabilities including development dependencies. These checks
  are not an external security assessment; Linux dependency warnings remain open.
- A 120-cycle release engine index/cache/query/impact soak passed in 52.2 seconds.
  Sampled working set stayed between 36.5 and 41.9 MiB on 1,000 files.

## Partially complete / external release work

Origin is configured to https://github.com/Rhythm-Sanghi/Threadmark.git. No hosted
release or GitHub CI execution is claimed. A manual Windows candidate workflow
checks and packages unsigned builds by default, accepting certificate secrets
later without code changes. The Windows build is unsigned, as authorized for
development/release candidates. Trusted public Windows signing remains blocked
until a real certificate exists. Enable private vulnerability reporting before
public distribution. Linux/macOS, multi-hour desktop memory and maximum-size
WebView frame profiling are unvalidated.
These are explicitly excluded from the local Windows validation claims.

## Known limitations and analysis blind spots

Static analysis does not resolve arbitrary runtime dispatch, reflection, computed
imports, framework mounts or dependency-injected services. Complex CommonJS,
block scope/destructuring/property mutation and dynamic Python imports can remain
unresolved. Relative local tsconfig inheritance is supported; package-based
extends is diagnosed rather than loaded from dependencies. Detected reassigned
JS identifier bindings produce diagnostics and suppress confident runtime calls.
No third-party dependency source or repository code runs.
Diagnostics, confidence and bounded-result markers expose these limits.

Git renames appear as added/removed identities. Changed files seed conservative
file-level impact; reachability is not a runtime prediction or test-coverage proof.
The local SQLite store includes evidence and cached syntax and is not encrypted.
The desktop uses platform-default storage; CLI supports a custom data directory.
Existing WebView2 is required; installer downloads are disabled. GUI logs go to
stderr rather than a persistent log file. Overview fit shrinks map text; Actual
size restores readable labels and List view provides full names and source lines.

## Performance measurements

Measured release build on Windows 11, i7-1365U, 16 GB RAM. A 10,000-file /
100,000-function fixture indexed cold in 7.40 s, refreshed unchanged in 3.52 s,
and updated one file in 4.60 s with 9,999 cached parses reused. Peak working set
was 731 MiB; database 319 MiB; warmed bounded search 42.6 ms, with separate first
adjacency load 973 ms. Refresh/update improved; query timings were slower in this
single run. Whole-graph resolution/JSON persistence remain large-index
costs. Full methods, raw results and native UI observation are in
[docs/performance.md](docs/performance.md); these are single-run fixtures.

## Next concrete task

Public distribution preparation: real signing certificate, private security
reporting and hosted CI execution. For larger repositories, prioritize normalized
graph persistence and affected
subgraph resolution using the committed benchmark before claiming faster updates.
