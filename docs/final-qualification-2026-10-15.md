# Final engineering review and local release-candidate qualification — 2026-10-15

This review starts at the original `main`/`v0.1.1` commit
`1ff28c900bd875c90a42183feac43ff2d4f2dc54`. The 11 hardening commits end at
`c5373aff4686fcca7a63161b616241c90bd2896b`. Two corrective local commits
follow: `b6bc719` preserves repository selection during background refreshes, and
`b38e1f2` records the desktop executable hash in release provenance. No branch,
tag, release, or PR was pushed or changed.

The final unsigned, local candidate is in the ignored workspace directory
`artifacts/final-qualified-b38/`. Its runtime
source is exactly `b38e1f234f15e3378f8374394252ae04bf52b7c7`. The
machine-readable [qualification evidence](final-qualification-2026-10-15.json)
contains all 240 final mixed-benchmark samples, build hashes, and disposable
upgrade results. The earlier [hardening handoff](hardening-2026-10-15.md) and
[evidence](hardening-2026-10-15.json) remain historical records.

## 1. Independent review of the original 11 commits

| Commit | Original limitation, implementation and test assessment | Remaining risk or scope limit |
| --- | --- | --- |
| `d355fde` | Discovery previously relied on a filter/sentinel and outer-loop coupling. Explicit visitor `Skip`/`Quit` bounds traversal and checks cancellation even across rejected entries. Cache UPSERT repairs same-hash malformed rows. Low-budget traversal, cancellation and repair tests reach the changed branches. | The prior sentinel was not shown exploitable. A 200,000-entry in-memory discovery list is bounded but can still consume substantial memory. |
| `85ca0b0` | Settings lacked Tab containment, node menus lacked arrow navigation and Escape focus return, and storage language obscured plaintext evidence. React handlers and tests address these. | DOM tests do not establish spoken screen-reader behavior or Windows text scaling. No visual redesign was introduced. |
| `57b1020` | Coarse indexing timing was unavailable; Windows opened-handle checks allocated a 64 KiB path buffer per read. Debug phase timing and a 512-character buffer with long-path retry address those limits; real long-path and boundary tests pass. | Phase timing combines read/parse work. No allocation profile or reliable large-graph speedup was established. |
| `eb523ee` | Working-tree source packaging could include untracked or edited files. `git archive`, a blob inventory, checksums, a clean-build receipt, and fixture tests bind packages to a commit. | The original receipt omitted the desktop executable; `b38e1f2` adds it. Local receipts are not signed attestations or binary reproducibility proof. |
| `bd1adb5` | Adds two actual narrow native Windows captures and labels the older frontend-rendered gallery accurately. This is documentation evidence, with no runtime change. | Images do not establish 200% zoom, screen-reader output, or general accessibility. |
| `400ce62` | Floating CI action tags and advisory-tool installation could drift. Workflow action SHAs, Node and cargo-audit versions are pinned; read-only permissions remain. YAML and local commands were checked. | GitHub-hosted CI for this unpublished branch has not run; pins need deliberate future maintenance. |
| `ee4b17d` | Separate snapshot/revision reads could pair stale graph data with a new revision; malformed cache JSON could be reused. One SQL statement and validation remove those cases. A concurrent writer test checks 100 publications. | Structurally plausible but maliciously altered cached nodes are outside the current validation contract. No schema change. |
| `1740060` | JS/TS explicit exports could lose to `export *`, and a late ambiguity left confident downstream barrel edges. Explicit precedence and reverse ambiguity propagation correct both; both regressions fail on original v0.1.1 and pass here. | Static resolution still has a 256-round fixed-point cap and cannot model arbitrary dynamic exports. Python behavior remains unchanged. |
| `6cc2e66` | Adds Windows junction boundary, long-path and exclusive-handle recovery regressions around the earlier traversal/read changes. These use actual filesystem behavior. | ACL denial and adversarial junction swapping were not physically reproduced. |
| `c43b470` | Blocking SQLite/graph queries and watcher joins ran on desktop commands; exports could truncate an existing destination. Worker commands and same-directory staged exports address those roots; injected write-failure tests pass. | Review found a separate selection race: comparison/watcher publication could restore an old repository. `b6bc719` corrects it. Disk-full/process-kill behavior is still not physically qualified. |
| `c5373af` | Records baseline/candidate tests, native captures, packaging and remaining gaps. Diff from `c43b470` is only `STATUS.md` and the two handoff/evidence documents. | It did not rebuild the original installer and did not itself change source, dependencies or packaging. Its test counts are historical after the new commits. |

The review inspected the changed source paths, transaction and cache boundaries,
re-export propagation, desktop command ordering, export staging, packaging and
workflows. The changes stay within the intended static-analysis and release
hardening scope. The UI retains Workshop, Gridline and Studio Utility styling.

## 2. Confirmed defects and corrections

**VERIFIED:** A Git comparison indexed on an independent `Engine` but published
the compared root as the global `repository` setting; the desktop command then
explicitly reselected that old root after comparison. The same persisted-setting
write occurred during native watcher refresh. If a user selected another repository
while those tasks ran, the background task could undo the selection. `b6bc719`
publishes comparison and desktop-watcher snapshots without changing that setting
and removes the desktop command's redundant reselection. Regression tests prove a
comparison leaves another instance's newer selection intact and a background
watcher does not select its root. The CLI watcher retains its original behavior,
with its own compatibility test.

**VERIFIED:** The build receipt did not hash `threadmark-desktop.exe`. `b38e1f2`
adds that artifact and requires it during packaging. A positive fixture packages
all three hashes; removing the desktop entry is rejected.

No other reproducible defect was confirmed in the reviewed commits. The known
full-graph resolution cost, static-analysis uncertainty and native gaps below
remain limitations rather than unverified claims of a fix.

## 3. Final automated checks

**VERIFIED:** Rust 1.98.1 `cargo fmt --all -- --check`, strict workspace/all-target
Clippy, and `cargo test --workspace --locked` pass: **54 tests**, none ignored.
Frontend lint, typecheck, 19 tests and production build pass. Release CLI and
Windows NSIS build pass; the package-provenance regression suite passes. Dependency
notices regenerate with no committed drift; license inventories are produced.
Cargo audit reports zero vulnerability advisories and retains the two previously
documented informational all-platform warnings. npm audit reports zero
vulnerabilities. These scans are not an independent security audit.

**VERIFIED:** The final package manifest identifies `b38e1f2`. All 127 archive
files match its recorded Git blob IDs, all four package checksums match, and the
CLI ZIP contains its executable, GPL license and dependency notices. Installed
license and notices match source bytes. The installer reports version 0.1.1 and
`NotSigned`; the CLI reports 0.1.1 and the migrated database reports schema 3.

## 4. Performance and uncertainty

Three alternating baseline/final-engine process pairs used the same release
configuration and 300 linked TypeScript modules, a barrel, consumer, ten tests
and `tsconfig`. Each process ran three warmups and five measured iterations per
case, yielding 15 samples per side. Cold indexing used a new SQLite database,
with the OS filesystem cache left intact. The `watcher_refresh` measurement
includes the deliberate 400 ms event debounce. All values are elapsed milliseconds;
parentheses give the full observed range. Raw samples are in the JSON evidence.

| Operation | v0.1.1 median (range) | Final median (range) |
| --- | ---: | ---: |
| Cold | 85.5 (75.1–128.8) | 77.8 (70.0–97.9) |
| Unchanged refresh | 60.0 (51.7–86.5) | 56.9 (51.0–75.3) |
| One-file update | 58.5 (52.9–133.3) | 57.4 (50.9–64.9) |
| File deletion | 54.8 (52.1–126.7) | 58.3 (52.8–64.0) |
| Re-export change | 119.9 (59.1–148.7) | 120.6 (58.4–152.0) |
| Import graph change | 123.1 (63.4–164.7) | 118.3 (110.9–142.0) |
| Configuration invalidation | 122.3 (90.1–166.0) | 123.5 (114.9–154.6) |
| Watcher refresh | 555.8 (498.7–614.1) | 548.9 (511.8–575.2) |

Median cumulative process peak working set was 22.5 MiB before and 22.0 MiB
after. Final SQLite size was 2.42 MiB on both sides, excluding transient journals.
Variability overlaps substantially. The earlier 10,000-file, independent-module
three-pair result remains: cold 6.23→6.54 s, unchanged 3.60→3.37 s, one edit
3.91→3.67 s. The 300-module mixed fixture does not reproduce that cold regression;
it does not resolve the large-workload concern. No general speedup is claimed and
no further performance code was introduced. Full-graph resolution and snapshot
publication remain future engineering work, best informed by allocation and
import-dense profiling.

## 5. Source, installer and license reconciliation

**VERIFIED:** The first runtime candidate was built from `c43b470`; the following
`c5373af` changed only STATUS/handoff/evidence. The final, corrected candidate
was rebuilt from `b38e1f234f15e3378f8374394252ae04bf52b7c7`, after both
review fixes. Its receipt and source manifest agree on that exact commit.

| Artifact | SHA-256 |
| --- | --- |
| Built CLI `threadmark.exe` | `990f62af4a3f4d66828bae1aef821fefe23f7ed1c9a40c772e6b37c189ad7253` |
| Built desktop `threadmark-desktop.exe` | `ddadc5e0b1709005d7c46f03728bf38a1ee124b58af032ec08c1ec6dbb7ef2cc` |
| Installer `Threadmark_0.1.1_x64-setup.exe` | `05ef0f1f2b79177f23cf52252843e03c738887b0176d339c3824597f95060563` |
| Source ZIP | `4e4b06530c4d685ef395caa8ac850a4a795a84293ad85e792fa9de04c6c4f2e8` |

The installed desktop hash is `e34b2d4554316cd95561cd8bb96690da63720895c7e4c6bec40e1fe5cd61ceb4`:
Tauri changes only its documented bundle marker from `UNK` to `NSS` in the NSIS
payload, and the test verified the resulting bytes. The installed CLI hash equals
the built CLI hash. These are local build/provenance checks, not a reproducible
build or signed supply-chain attestation. Any later documentation-only commit
must retain `b38e1f2` as the binary's source identity; it must not relabel it.

## 6. Genuine previous-version upgrade and recovery

**VERIFIED:** The available v0.1.0 NSIS installer was installed into an isolated
`artifacts` path. Its CLI built a valid two-file index in a disposable data
directory, selected the repository, saved `dark`, and reported schema 2. Installing
the final candidate over it migrated to schema 3, preserved theme and selection,
and returned two search hits. The installed CLI and desktop payloads matched the
recorded build hashes (accounting for the Tauri marker). Uninstall removed the
test executable and registration but left the database byte-for-byte unchanged;
reinstall reopened it with schema 3, theme and repository intact. The temporary
test installation was removed and the earlier test registration restored. The
real user application database was not used for destructive testing.

**VERIFIED, bounded failure checks:** An intentionally corrupt *copy* of the
database produced a clear SQLite error and remained unchanged. Two simultaneous
CLI instances indexed a separate two-file fixture successfully, leaving a usable
schema-3 database. Existing tests inject cache-write failure, export-write failure
and exclusive source-handle denial. Package regression tests reject stale output,
dirty source, missing/incorrect/partial receipts and changed binary hashes.

**NOT VERIFIED:** A physical disk-full condition, interrupted NSIS installation,
process-kill crash recovery, ACL denial, and concurrent *desktop* instances were
not run. Database corruption is reported, not automatically repaired. The source
ZIP is a developer/release artifact; Threadmark does not ingest a source archive
at runtime (**NOT APPLICABLE** to the indexing workflow). Hash and blob checks
cover the produced ZIP; a damaged downloaded copy should fail checksum checking.

## 7. Native desktop and accessibility

**VERIFIED:** The earlier `c43b470` native session reindexed a fixture, searched,
inspected the map and source, started/stopped watching, exported metadata through
the Windows dialog, and checked a narrow map and Settings. The final runtime was
also launched as an actual Windows executable; the welcome screen and labeled
controls appeared through Windows accessibility inspection. The final installer
installed byte-verified desktop and CLI binaries.

**NOT VERIFIED:** The complete open→index→search→evidence→impact→Git changes→tests
→export→reopen flow, rapid switching while queries run, 200% WebView zoom,
Windows text scaling, long-name layout, spoken NVDA/Narrator output, and a
sustained native workload were not repeated on the final build. The native test
helper lost the target window after user input/minimization. More critically, a
`LOCALAPPDATA` environment override did not redirect the desktop's Windows known
folder: it opened the existing profile database. We stopped before destructive
native operations. Automated DOM assertions and an accessibility tree are not
spoken-output verification. The earlier 51-second engine soak is not a native
endurance test.

## 8. Security and release status

**VERIFIED:** Static analysis reads local source without running repository code.
Opened-handle source checks, traversal limits, cache repair, atomic SQLite
publication and default evidence-free exports remain in place. Local DB, settings,
cached syntax and source evidence are plaintext as documented. No telemetry,
accounts, repository-script execution, source upload or new network service was
introduced by these commits. The signed-release path was inspected but not used.

**BLOCKED:** Trusted Authenticode signing requires an authorized publisher
certificate. Qualification on a stable Windows build and a fresh Windows user
profile requires another host/profile; this host runs Windows 11 Pro Insider
Preview 10.0.26220. Public production qualification and an external security or
accessibility audit are not established. No release was published.

## 9. Documentation and presentation

The README remains concise and describes the local Rust/React boundary, static
limits, languages, install and developer commands. STATUS now points to this
review and separates the local candidate from the published unsigned prerelease.
The release guide now describes the installer script's same-version lifecycle
check accurately and links to this genuine prior-version upgrade. Historical
records and the existing visual design were preserved; no marketing text, badge,
invented statistic or generated screenshot was added.

## 10. Branch and PR reconciliation

At review time, remote `main` and `v0.1.1` still identify `1ff28c9`.
`cleanup/repository-presentation` and draft [PR #1](https://github.com/Rhythm-Sanghi/Threadmark/pull/1)
are at `0d7b783`. Their common ancestor with this branch is `1ff28c9`.
PR #1 consolidates architecture decisions, moves six historical JSON records
under `docs/evidence`, and updates links. Git patch-equivalence inspection found
no duplicate commit. A final local `git merge-tree` rehearsal found one content
conflict in `docs/release.md`: PR #1 updates the historical lifecycle-report
link to `evidence/installer-lifecycle-windows-0.1.1.json`, while this branch
corrects the nearby upgrade-script wording. Resolve it by retaining this branch's
wording and PR #1's moved evidence link. No runtime file conflicts were reported.

Proposed sequence: review and merge PR #1 once; update this branch through an
ordinary merge of the resulting `main`; resolve that one link/wording conflict;
rerun link/build checks; then open a separate hardening PR. Do not force-push,
merge automatically, delete the cleanup branch, alter tags, or change branch
protection.

Proposed PR title: **Harden Threadmark indexing, desktop responsiveness, and
release provenance**.

Proposed PR description:

> Bound discovery explicitly, repair corrupt parse caches, read snapshots with
> their revisions, and correct ambiguous JS/TS re-export relationships. Move
> desktop queries and exports off the UI thread, preserve repository selection
> during background refreshes, improve keyboard focus, and bind release packages
> to committed source plus CLI/desktop/installer hashes. Verified with 54 Rust
> tests, 19 frontend tests, packaging regressions, Windows candidate build, and
> an isolated v0.1.0→0.1.1 upgrade. The installer is unsigned; native screen-reader,
> 200% scaling and sustained stability remain open qualification work.

## 11. Local Git state

Runtime fix: `b6bc7195990a834731685ad2b7d54a33cc74d620`.
Release-provenance fix and installer source: `b38e1f234f15e3378f8374394252ae04bf52b7c7`.
This review is documentation-only and follows the runtime commit. The final
working tree and diff checks are recorded after it is committed. No remote write
was made.

## 12. Exact remaining blockers

- **BLOCKED:** trusted signing without an authorized publisher certificate;
  stable-Windows/fresh-profile qualification on this Insider host.
- **NOT VERIFIED:** spoken screen-reader output, 200% WebView and Windows text
  scaling, final-build complete native workflow, multi-hour native endurance,
  physical disk-full/interrupted install/crash recovery, and concurrent desktop
  instances.
- **NOT VERIFIED:** allocation/import-dense profiling for the 10,000-file cold
  regression; full-graph refresh remains costly. An external security review and
  independent binary reproducibility assessment are also absent.
- **VERIFIED:** local source/tests, disposable prior-version upgrade, installer
  payloads, manifest/blob/checksum linkage, and scoped recovery tests above.
