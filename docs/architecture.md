# Architecture

threadmark-db owns transactional SQLite migrations, settings, repository snapshots
and per-file parse caches. threadmark-core owns discovery, Tree-sitter adapters,
resolution, adjacency queries, impact, Git comparison, editor configuration and
watching. CLI and Tauri commands call that engine; React contains interaction and
layout code, never parsing or traversal. No external service is required.

Adapters return common nodes, imports, references, exports and diagnostics. IDs
hash repository, path, language, qualified name and kind, excluding line numbers.
Resolution emits directed edges with confidence, source spans, evidence and adapter.
Missing/ambiguous/runtime targets remain diagnostics. Graph adjacency loads lazily
in Rust. Clients request bounded views and versioned responses.

SQLite schema 3 uses repositories, parsed_files, settings and index_revisions. A
small revision table invalidates query caches across connections without loading
the JSON snapshot on every query. Engines keep their selected root when another
process selects a different repository. Schema 2 upgrades preserve stored data.
Snapshots are
JSON, and cached syntax is JSON per file with hash and adapter version. This keeps
persistence small in implementation, but whole graph serialization, re-resolution
and snapshot loading are measurable costs at large scale (performance.md).
Unchanged cache rows are retained; deleted paths are purged transactionally. All
snapshot/cache/settings publication is one transaction. Cancellation before save
leaves the previous snapshot intact. Future normalized graph storage can replace
this persistence boundary without moving analysis into clients.

Desktop workers index/compare off the UI thread using their own engine connection.
Query state is serialized behind a mutex. Watch updates reload query adjacency and
emit only overview metadata. SQLite waits up to five seconds for busy writers.
Startup errors remain visible and retryable. The desktop runtime contains local
assets and restricted IPC capabilities. No shell, HTTP, updater or telemetry plugin
is installed. Versioned contracts are shared DTOs, not rusqlite representations.

## Decisions

### Local analysis only

Accepted. The application must work offline after installation. Rust owns source
analysis. No accounts, telemetry, cloud indexing or external AI dependencies.
Dependency downloads and developer tooling are separate from runtime behavior.


### Deterministic symbol identity

Implemented. BLAKE3 hashes length-prefixed repository root, relative file path,
language, qualified symbol name and kind. Source line numbers do not participate;
moving a declaration within its file preserves its ID. Renaming or moving a file
produces added/removed IDs. Duplicate declarations are diagnosed and excluded
from confident name resolution. Indexing and watcher fixtures check identity.

Content fingerprints normalize CRLF to LF for local Git comparison; byte-sensitive
file hashes still invalidate parse caches and preserve accurate source spans.

### Do not execute repository code

Accepted. Repository contents are untrusted input. Ordinary indexing must never
run project package scripts, executables, tests or build commands. Developer
commands in README build Threadmark itself, not repositories it analyzes.


### Load bounded graph slices

Implemented. Rust serves directed neighborhoods and evidence paths. React receives
a bounded slice, rather than loading or traversing the complete graph. Desktop
neighborhood requests use 24 nodes and one to three hops. Core slice requests are
capped at 120 nodes. Limits are marked so an incomplete view cannot imply that
there are no further relationships. Navigation refocuses or expands on demand.
