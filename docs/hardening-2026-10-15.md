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

Implementation and final qualification results follow below when measured.
