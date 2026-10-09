# Incremental indexing and watching

`threadmark watch <path>` watches local files until Ctrl+C. `--json` emits one
versioned overview per line. Desktop Watch changes starts an optional watcher;
Stop watching, opening another repository and manual reindex stop it. Git
comparison also stops watching before refreshing.

Native notify 8.2 events coalesce after 400 ms quiet, with a two-second ceiling
for long bursts. The event channel holds 64 notifications; overflow is reported
and forces refresh. Generated-directory events and read-only access events are
excluded. Source/config/ignore changes, directory changes and Git HEAD/index/refs
changes trigger discovery, hashing and selective reparsing. Relationship resolution
is conservatively rebuilt for the complete current graph, so renamed/deleted files
and changed aliases invalidate existing relationships. Cache keys include adapter
version and content hash. Transactional save publishes a complete index.

Watching never executes a repository command. Backend/read errors are visible.
Cancellation preserves the previous index. Native notifications may be unreliable
on remote or virtual filesystems; use Re-index when needed. The backend can receive
large generated-folder bursts even though they are filtered before indexing.
There is no polling fallback. Data directories should be outside the repository.
