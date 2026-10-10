# Security boundary

Repositories are untrusted input. Ordinary indexing uses static source/config
reads and Tree-sitter; it never invokes package managers, scripts, builds, tests,
Python, Git hooks, external diffs, filters or repository executables. Git reads
local objects with libgit2, with network features disabled. Project configuration
is parsed as data; no arbitrary plugins or inherited executable settings load.

Discovery excludes generated paths and rejects symlinks/reparse points. Every
source read rejects traversal, checks each component and canonical root boundary.
On the validated Windows platform, the opened file handle's final path is verified
before bytes are read, preventing an outside-root junction swap between checks.
The root itself is explicitly selected/canonicalized. Other platforms are not
validated for release. No external-source following option exists.

Budgets: 64 directory levels, 200,000 discovery entries, 50,000 examined files,
2 MiB per file, 512 MiB source bytes, 300,000 AST nodes and two seconds per parse,
200,000 graph nodes, 500,000 syntax references/relationships. Limits and malformed
input yield explicit errors or partial-index diagnostics. Query/impact budgets
are documented separately. Native notifications have a bounded event queue.

Editor launch requires an explicitly configured absolute installed executable.
Arguments are passed directly, with no shell and no repository working directory.
Only an explicit Open in editor action launches it. React escapes source text.
CLI human output removes terminal control characters. Metadata JSON export omits
source evidence unless explicitly requested. No .env values are indexed. Literal
process.env.KEY syntax records key names and reference spans only; parameter
shadowing is excluded. Global runtime rebinding remains a static-analysis limit.

The local SQLite store contains source evidence snippets and cached syntax; it
inherits user OS permissions and is not encrypted. Repository paths/usernames
appear in JSON. No source is uploaded and no telemetry libraries are included.
Production CSP connects only to local IPC; there are no hosted fonts or remote
assets. File export uses an explicit native save dialog. GPL and third-party
license texts accompany the installer. See SECURITY.md for reporting.

Default metadata export also clears free-form diagnostic messages, because parser and dynamic-call errors can contain source expressions. Diagnostic codes, paths and spans remain. Explicit CLI --include-evidence includes messages as well as edge snippets.

The 2026-10-07 dependency advisory check found zero Rust vulnerability advisories
and zero npm vulnerabilities. The all-platform lockfile carries informational
warnings RUSTSEC-2024-0370 (unmaintained proc-macro-error 1.0.4) and
RUSTSEC-2024-0429 (unsound glib 0.18.5); neither package is present in the Windows
target graph. These remain relevant before a Linux release. This dependency
check does not establish that the application is free of security defects.
See [dependency advisory summary](evidence/security-audit-windows-0.1.1.json).
