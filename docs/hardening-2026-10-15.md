# Engineering hardening — 2026-10-15

## Baseline, before implementation

Checkout: clean `main`, `1ff28c900bd875c90a42183feac43ff2d4f2dc54`,
identical to local and remote `v0.1.1`. Public GitHub API confirms an unsigned
prerelease with installer, CLI/source ZIPs, signing status and checksums. This
confirms asset metadata, not a rebuilt-binary equivalence audit.

Draft PR #1 (`0d7b783fb8040486370242e0be669f7415ab0c7c`) consolidates four
decisions into architecture.md and moves historical JSON into docs/evidence.
Those changes are absent locally. Leave that work intact; reconcile this branch
with that PR through an ordinary merge after owner review. No history rewrite.

Confirmed defects:

- `scripts/package-release.ps1` copies tracked **and untracked** working files,
  so its source ZIP does not establish commit provenance. Existing release
  binaries cannot be proved to match their source simply from the tag.
- `threadmark-db::Database::save_index` refuses to replace a cache row when its
  hash/version are unchanged, even if core reparsed malformed JSON. Cache repair
  is therefore repeatedly lost. Transaction atomicity is already implemented.
- `App.tsx` Settings has initial/restored focus but no Tab containment;
  its context menu lacks arrow-key navigation and focus restoration.

Boundary review: indexing's filter returns true after 200,000 visits, but the
outer loop checks the shared counter before using the yielded entry. This does
not establish an exploitable symlink escape. The coupling is fragile, counts
the root, and cancellation cannot interrupt a long run of rejected entries.
Replace it with explicit traversal termination and low-budget regression tests.

Performance: every refresh deserializes cached parses, resolves all files,
detects cycles, serializes the snapshot and publishes it. Existing benchmark
measures whole operations, not allocations or each internal phase. Capture a
fresh controlled baseline before optimizing; historical numbers are single runs.

Security/privacy: source reads check components and Windows opened-handle paths;
no code execution or telemetry found in inspected engine/CLI/Tauri paths.
Local evidence is plaintext as documented. No external security audit performed.
Source-package provenance is a confirmed risk; no new exploit is asserted.

Documentation: README and STATUS correctly describe static scope and dated
verification. Preserve historical evidence. Signing and multi-hour desktop,
screen-reader, Windows scaling and low-disk qualification remain unverified here.
New component libraries, graph-store rewrites and decorative UI changes are
optional and need evidence before adoption.

Baseline checks: `npm ci` (189 packages audited, zero npm vulnerabilities),
`npm run lint`, `npm run typecheck`, `npm test` (17), `npm run build`, and
`cargo test --workspace --locked` (38) passed. Existing dist was present for the
initial Cargo run; frontend was then rebuilt. Toolchain: Rust 1.98.1, Node 24.15.0,
npm 11.12.1. Ignored target, artifacts, node_modules, dist and generated icons
were present and preserved. No AGENTS.md was found in the repository inventory.

Final qualification results follow below.

## Engineering handoff

### 1. Verified changes

- Discovery now uses explicit `WalkState::Skip`/`Quit`, with one traversal thread.
  Rejected entries count toward the budget; the root does not. Low-budget tests
  cover exact/over limits, excluded subtrees, depth and Windows junctions. The
  old outer-loop check prevented processing the filter's sentinel; no escape
  exploit was demonstrated. Cancellation is checked in the visitor and parsing.
- Malformed or structurally empty parse-cache rows are reparsed and repaired.
  A same-hash cache repair can now replace its invalid SQLite row. Integration
  tests verify the following unchanged refresh reuses the repaired cache.
- Snapshot and revision load in one SQLite statement. This removes the selection
  race that could cache an old snapshot under a newer revision. A concurrent
  writer fixture verifies 100 publications remain paired. No schema change.
- Cancellation after serialization is honored before persistence. Failed writes
  roll back snapshot, cache and revision together; cancellation/error tests compare
  the exact previous snapshot, including after reopening. Cancellation arriving
  after the publication boundary does not interrupt an atomic commit.
- Explicit JS/TS exports override star re-exports. Late ambiguities are retracted
  through downstream barrels. Both new regressions fail on the original v0.1.1
  engine and pass after correction. Python precedence was deliberately unchanged.
- Desktop queries and watcher start/stop execute on blocking workers, keeping
  graph loads, SQLite waits and watcher joins off the UI thread. Watch lifecycle
  mutations stay serialized. Export serialization also runs on a worker.
- Desktop exports stage a same-directory temporary file, sync it and publish it
  by replacement. Injected write failures preserve an existing destination and
  leave no staging file. This is fault injection, not a physical low-disk test.
- Settings contains Tab focus and restores it when closed. Node menus support
  arrows/Home/End and restore focus on Escape. Search identifies its active option.
  Local-storage text states what is cached and that the database is unencrypted;
  first use explains static reachability's limits.
- Source packaging uses committed Git blobs, records commit/blob inventory and
  hashes, and excludes working edits/untracked files. Binary packages require a
  matching build receipt and empty output directory. Regression fixtures cover
  dirty/untracked/ignored input, dotfiles, repeatability and missing/wrong receipts.
- CI retains read-only repository permissions, pins action revisions and Node,
  pins cargo-audit 0.22.2, checks notice drift and runs source-packaging regressions.

### 2. Code inventory

| Files | Purpose |
| --- | --- |
| crates/threadmark-core/src/indexing.rs | Explicit bounded traversal, smaller Windows handle-path buffer with long-path retry, phase timing |
| crates/threadmark-core/src/lib.rs | Cache validation, post-serialization cancellation, coherent graph revision loads, phase timing |
| crates/threadmark-core/src/resolution.rs | Explicit-export precedence and downstream ambiguity retraction |
| crates/threadmark-db/src/lib.rs | Cache repair, snapshot/revision pairing and transactional regression tests |
| crates/threadmark-core/tests/{graph,persistence,resolution}.rs | Boundary, snapshot preservation, clean-index equivalence and analysis regressions |
| crates/threadmark-core/examples/benchmark.rs; Cargo.toml; Cargo.lock | Opt-in timing logs; existing tracing-subscriber dependency reused |
| apps/desktop/src/{App.tsx,App.test.tsx} | Focus, menu, combobox and privacy/onboarding text |
| apps/desktop/src-tauri/src/{main.rs,export.rs}; Cargo.toml | Worker commands and atomic export with failure tests; existing tempfile package reused |
| scripts/{build-candidate,package-release,sign-release,test-package-release}.ps1 | Commit-linked candidate build and packaging checks |
| .github/workflows/{check,release}.yml | Packaging smoke checks, pinned actions/tooling and notice-drift rejection |
| README.md; STATUS.md; docs/release.md; docs/screenshots/ | Architecture/procedure corrections and genuine native captures |

No fixtures, legal notices or historical evidence were removed. Third-party
notices were regenerated and are unchanged. No new package version was introduced;
tracing-subscriber and tempfile already existed in the lockfile.

### 3. Performance

| Files | Operation | v0.1.1 median ms (range) | Hardened median ms (range) |
| ---: | --- | ---: | ---: |
| 100 | Cold | 88.7 (79.1–114.9) | 78.8 (62.9–81.6) |
| 100 | Unchanged | 38.5 (37.8–69.6) | 37.8 (34.1–41.2) |
| 100 | One-file update | 82.0 (47.5–123.5) | 45.4 (37.7–46.4) |
| 1,000 | Cold | 680.0 (679.4–851.3) | 639.3 (633.2–916.7) |
| 1,000 | Unchanged | 386.5 (384.3–386.5) | 373.5 (310.8–404.7) |
| 1,000 | One-file update | 383.7 (364.6–449.2) | 348.3 (333.2–372.0) |
| 10,000 | Cold | 6,229.4 (6,208.9–9,020.2) | 6,538.9 (6,039.4–6,709.5) |
| 10,000 | Unchanged | 3,598.8 (3,453.3–4,975.3) | 3,366.9 (3,311.9–3,396.8) |
| 10,000 | One-file update | 3,914.5 (3,741.2–5,668.8) | 3,666.5 (3,628.7–3,668.2) |

| Files | Median peak MiB, before / after | SQLite MiB, both |
| ---: | ---: | ---: |
| 100 | 19.4 / 19.8 | 3.2 |
| 1,000 | 84.4 / 85.1 | 31.9 |
| 10,000 | 728.9 / 730.3 | 319.3 |

Peak working set is cumulative within each process across the three sizes,
not an isolated per-workload measurement. SQLite sizes exclude transient journals.

Machine: Windows 11 Pro Insider Preview 10.0.26220 x64, Intel Core i7-1365U, 12 logical processors, 15.66 GiB
reported RAM (16 GB nominal), Rust 1.98.1 optimized builds, Node 24.15.0.
The Microsoft SDK was absent from the system installation; SDK CPP/CPP.x64
10.0.26100.3916 was extracted under ignored artifacts/sdk and used with installed
MSVC 14.44.35207. No system SDK installation was changed.

Method: the existing benchmark, three alternating baseline/changed process pairs,
100/1,000/10,000 generated TS files, ten functions/file, no inter-file imports.
The baseline was rebuilt from v0.1.1 with the same compiler/SDK. Each cold operation
uses a new database, not a flushed filesystem cache. No build or native UI action
ran during these qualification pairs. Every result preserves expected counts,
cache reuse and bounded reachability. Raw before/after runs and phase profile are
in [hardening evidence](hardening-2026-10-15.json). Earlier exploratory runs that
overlapped compilation are excluded from the qualification comparison.

At 10,000 files, median unchanged refresh moved from 3.60 s to 3.37 s and one-file update from 3.91 s to 3.67 s, while cold indexing regressed from 6.23 s to 6.54 s. Variability is substantial, especially the first baseline run. Memory and storage are effectively unchanged. These observations do not establish a general speedup; the safer full-resolution architecture is retained.

The smaller handle buffer reduces its usual allocation from 65,536 to 1,024 bytes
per source read; a real long-path test verifies retry and size enforcement. This
is not a claim about total process allocation. Timing logs separate cache load,
discovery, combined discovery/read/parse, resolution, cycles, serialization and
SQLite publication. Combined discovery/read/parse includes discovery and snapshot-node construction time; do not
sum both. Allocation profiling and separate read/hash/deserialization/parse costs
were not captured. Whole-graph resolution and JSON persistence remain. No
normalized store or affected-subgraph cache was introduced without evidence of
reliable benefit.

Additional functional timing observations (CLI startup + indexing + overview, new database followed by two unchanged runs; no baseline comparison or peak-memory measurement):

- react-source: 10 files, 135 symbols, 414 relationships, 821 diagnostics; elapsed 108.6, 36.5, 35.4 ms; SQLite 0.74 MiB. Both refreshes reused every parse.
- python-email: 29 files, 1529 symbols, 4175 relationships, 4104 diagnostics; elapsed 726.9, 92.8, 100.4 ms; SQLite 5.70 MiB. Both refreshes reused every parse.

The React workload is Threadmark apps/desktop/src. Python is the installed CPython 3.11 email package, not an independently selected application repository. These source scopes contain many external/dynamic references; the diagnostic counts expose limited static resolution and are not runtime-coverage claims. No source files were modified and no workload code/tests were executed. Malformed/unsupported files, imports, deletion/renaming and configuration changes are covered by the clean-index equivalence regression.

The final engine soak completed 120 index/cache/adjacency/impact cycles over 1,000 files and 10,000 functions in 51.31 s. Sampled working set ranged 36.6–42.8 MiB; final 42.8 MiB. Assertions checked cache reuse, node counts and eight-hop reachability. This approximately 51-second engine run is not multi-hour native stability evidence.

### 4. Quality checks

| Command | Result |
| --- | --- |
| cargo fmt --all -- --check | Pass |
| cargo clippy --workspace --all-targets --locked -- -D warnings | Pass |
| cargo test --workspace --locked | Pass: 53 tests, none ignored |
| cargo build --release -p threadmark-cli --locked | Pass (candidate builder) |
| npm ci (apps/desktop) | Pass: locked install |
| npm run lint | Pass |
| npm run typecheck | Pass |
| npm test | Pass: 19 tests |
| npm run build | Pass |
| npm run tauri build | Pass: Windows x64 NSIS installer |
| scripts/build-candidate.ps1 | Pass: commit/hash receipt |
| scripts/package-release.ps1 -OutputDirectory artifacts/hardening-qualified-candidate | Pass |
| scripts/test-package-release.ps1 | Pass |
| scripts/verify-installer.ps1 | Pass: same-version 0.1.1 install/uninstall/reinstall; data preserved |
| scripts/audit-licenses.ps1 | Pass: inventories produced |
| scripts/generate-notices.ps1 | Pass: no committed notice drift |
| cargo audit --json | Zero vulnerability advisories; two informational warnings |
| npm audit --json | Zero vulnerabilities |

Original source with the new re-export tests: two expected failures, recorded in
artifacts/baseline-resolution-regressions.txt. These are baseline reproductions,
not failures of the final checkout. Initial rebuild attempts failed for missing
link.exe/SDK headers; final checks used the local SDK environment and passed.
The initial 38-test baseline used preexisting compiled artifacts; the later
baseline benchmark and all changed-source checks were rebuilt.

### 5. Desktop qualification

The final c43b470 Windows executable opened an existing fixture, re-indexed nine source files, searched createForecast, displayed its nine-node neighborhood/source preview, and started/stopped the native watcher. Saving through the native JSON dialog produced 20 nodes and 40 edges; file inspection confirmed all evidence/message fields were empty and no staging files remained. Earlier wide/narrow native captures verified Fit selection, Settings path wrapping and visible reverse-Tab focus wrap from Close settings to Configure editor. The capture helper intermittently returned stale accessibility data or unavailable geometry. A post-save capture showed another foreground app and was discarded; it is not native evidence. Native outcomes were verified through actual app states and the output file, not component mocks.

Automated component tests use mocked Tauri responses. Engine/CLI tests use actual
temporary files and databases. The native checks use the actual Windows executable
and WebView, not frontend recordings. The preexisting high-density PNG gallery is
explicitly labeled as historical frontend captures; the new JPEGs are native.
No independent external validation or screen-reader audit occurred. 200% UI zoom,
Windows text scaling, physical low-disk handling, process-kill crash recovery and
multi-hour desktop endurance remain unqualified. The unreadable-source regression
uses a Windows exclusive handle, not an ACL-denial fixture. A separate Unix symlink
fixture was not run on Windows; junction boundary tests did run.

### 6. Security and privacy

Source analysis remains local and static. No telemetry, source uploads, runtime
inference, accounts, execution hooks or repository test execution were added.
Opened-handle boundary verification and budgets remain in place. Local evidence
remains plaintext and default exports omit evidence/messages. Phase logs contain
phase names and timings, not source snippets. Temporary-export failure cleanup is
covered. No encryption infrastructure or source-deleting cleanup feature was added.

The current Rust scan reports no vulnerability advisories. Informational warnings
RUSTSEC-2024-0370 (proc-macro-error 1.0.4) and RUSTSEC-2024-0429 (glib 0.18.5)
remain in the all-platform lockfile; Windows target reachability was checked
separately. These results are not an external audit or proof of vulnerability
absence. Physical fault/crash tests and a comprehensive hostile-input audit remain.

### 7. Interface and developer experience

Workshop/Gridline surfaces, typography, colors and compact panels are preserved.
Changes address keyboard operations, uncertainty wording and cache transparency;
no component library or decorative UI redesign was added. Native narrow map and
Settings captures demonstrate usable controls and wrapped storage paths. The
README now explains the engine/client boundary. Release instructions explain
clean source, immutable archives, receipts and new output directories. Existing
history and the draft cleanup PR's reorganization were preserved.

### 8. Release status

- Source: passes the listed local checks; runtime candidate commit is
  c43b470417fc1716c064c6e51ed1f29b2fd605bb. The final documentation/evidence commit
  follows it and does not change runtime code.
- Windows candidate: unsigned installer, CLI/source ZIPs, manifest and checksums
  are in artifacts/hardening-qualified-candidate. The manifest identifies the
  exact source commit; this is a local candidate, not the published 0.1.1 binary.
- Signed release: blocked on a real authorized publisher certificate. The signing
  path validates private key, EKU and chain by inspection; it was not exercised
  with a real signing certificate.
- Public production qualification: not established. No release was published.

Published v0.1.1 source was downloaded and compared with its tag: all 119 files
match exactly or after CRLF normalization; no extra/missing/different files.
SHA-256 2d54ac418b8f4fc8f15d89f2ecfc046f75ada15fc8105945c5708e35e4b1c539
matches GitHub asset metadata. This does not establish binary reproducibility.

### 9. Git and PR #1

Branch: codex/engineering-hardening. All work is local; no push, merge, release,
remote deletion or GitHub setting change was performed. The final working tree
is checked after the documentation commit. Runtime commits:

```text
d355fde Harden discovery termination and repair cache publication
85ca0b0 Improve keyboard focus and local-data explanations
57b1020 Profile indexing phases and reduce Windows path-check allocation
eb523ee Package immutable source and verify candidate build provenance
bd1adb5 Record genuine native Windows map and Settings captures
400ce62 Pin CI actions and advisory tooling to reviewed revisions
ee4b17d Load graph snapshots and revisions atomically and reject invalid caches
1740060 Respect explicit exports and retract ambiguous barrel relationships
6cc2e66 Verify limited junction discovery and unreadable-source recovery
c43b470 Run desktop queries off the UI thread and publish exports atomically
```

[PR #1](https://github.com/Rhythm-Sanghi/Threadmark/pull/1) remains a separate draft
at 0d7b783fb8040486370242e0be669f7415ab0c7c. Review and merge it once, then merge
this hardening branch normally. Resolve documentation overlap by retaining its
evidence moves and the updated packaging instructions. Do not duplicate its
moves, rewrite shared history, discard its branch or change published tags.

### 10. Remaining work, in priority order

1. Qualify on a stable Windows build and fresh installation; this host runs
   Windows 11 Pro Insider Preview. Complete native accessibility qualification with a screen reader, 200% zoom
   and Windows scaling. Run sustained desktop sessions and measure handles,
   threads and WebView frame times; the engine soak does not cover those.
2. Exercise physical disk-full, process termination during SQLite publication,
   ACL-denied trees, watcher-overflow delivery, rapid repository switching and
   concurrent native/CLI operation beyond the bounded automated fixtures.
3. Profile allocations and imported/dense real-world graphs before designing
   affected-subgraph updates or normalized persistence. Current refresh still
   rebuilds relationships; the measured costs and regressions remain visible.
4. Qualify a fresh 0.1.0-to-current installer upgrade separately. Schema-2 copies
   are migration-tested; this run's native install started at 0.1.1.
5. Obtain a real signing certificate and independent external/user validation.
   Review all-platform dependency warnings before any non-Windows release.

These are validation gaps or external requirements, not asserted exploit findings.
The broad assignment is not represented as exhaustive qualification of every
listed platform/input/lifecycle condition.

### 11. Suggested GitHub metadata

Description: Local codebase relationship maps and static change impact for
JavaScript, TypeScript, and Python.

Topics: rust, tauri, static-analysis, code-navigation, typescript, python, windows.
Owner approval is still required to apply public metadata; nothing was changed.

### 12. Verdict

A materially hardened, locally verified Windows release candidate with stronger
analysis correctness, storage consistency, export safety and source provenance.
It retains full-graph refresh costs and explicit validation gaps. The evidence
supports an improved early release candidate, not perfection, an external audit,
a signed release or broad production qualification.
