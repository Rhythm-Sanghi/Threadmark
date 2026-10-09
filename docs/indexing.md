# Indexing

Choose a local directory in the desktop, or run threadmark index <path>. Discovery
honors repository .gitignore and .threadmarkignore gitignore syntax; parent/global
ignore files and Git exclude are intentionally disabled. Built-in exclusions cover
node_modules, .git, dist, build, coverage, .next, vendor, target, .venv and __pycache__.
Windows reparse points/symlinks are excluded and reported. Hidden source files are
eligible. Source/config contents are never executed.

Caps: depth 64, 200,000 discovered entries, 50,000 examined files, 2 MiB/file,
512 MiB total UTF-8 source, 300,000 AST nodes and two seconds per syntax parse,
200,000 graph nodes, 500,000 syntax references/relationships. Limits/skips, invalid
UTF-8, I/O/configuration errors and malformed syntax produce diagnostics. Partial
analysis stays visible. Unsupported source languages are identified explicitly.

Progress reports discovery/parsing/cache reuse and saving. Cancellation checks
between files and before publication; a parse has its own two-second timeout.
Cancelled jobs leave the previous usable index. Content hashes and adapter version
control reuse; every refresh rebuilds relationship resolution. Watch mode reuses
this same path. SQLite save atomically publishes snapshot and cache metadata.

SQLite user_version migrations run under an immediate write transaction. Newer
schemas are rejected without downgrading; failed migration rolls back. Local
settings and caches are outside repositories by default. The desktop exposes recent
repositories and selects their saved snapshot; Re-index refreshes it. Source preview
warns when content differs from that snapshot.
