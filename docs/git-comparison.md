# Local Git comparison

`threadmark changes --against HEAD` and `threadmark impact --against main`
refresh source analysis and compare it with a locally available commit/ref.
The desktop Changes panel uses the same operation. No checkout, hooks, external
diff command, filter, fetch or repository script runs. Non-Git folders and missing
references return explicit errors.

Source identity excludes line numbers; declaration fingerprints detect changes.
Path changes currently appear as removed and added identities. Relationship
comparison ignores span shifts. Baseline objects use baseline project configuration
and ignore files. Untracked and unstaged source files participate in current analysis.
Deleted identities use baseline reverse paths, whose evidence may describe files
that no longer exist. Changed files lists source analysis changes, not all Git
metadata or unsupported-language changes. Comments within declarations count as
changes. Traversal limits remain visible in JSON.

Comparison returns at most 2,000 symbol changes and 2,000 added/removed relationships per list, with full counts and a truncated flag. Impact takes at most 2,000 targets. File changes conservatively seed declarations in that file, including unchanged declarations; symbol-change lists only count changed declaration fingerprints. This can produce broader reachability than an individual symbol query.
