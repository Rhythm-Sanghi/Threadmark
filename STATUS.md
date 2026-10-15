# Project status

Threadmark 0.1.1 is an early Windows release candidate. Source is available at
[Rhythm-Sanghi/Threadmark](https://github.com/Rhythm-Sanghi/Threadmark).
The installer, portable CLI, source and checksums are available in the
[0.1.1 prerelease](https://github.com/Rhythm-Sanghi/Threadmark/releases/tag/v0.1.1).

## Working features

- Static JavaScript, TypeScript and Python indexing, search and source previews.
- Dependency maps, relationship evidence, directed paths and reverse impact.
- Local Git comparisons, including evidence for removed symbols.
- Related test discovery, Express routes, JSX references and configuration keys.
- Incremental parse caching, file watching, cancellation and persisted settings.
- Windows desktop app, CLI, unsigned installer and source packaging.

## Checks

The local 2026-10-15 hardening candidate passed 53 Rust tests and 19 frontend
tests, strict Clippy, formatting, frontend lint/type checks and Windows installer
build checks. Same-version install, uninstall and reinstall preserved local data.
See the [engineering handoff](docs/hardening-2026-10-15.md) for exact scope,
measurements, native checks and remaining gaps. These source changes have not
been published as a replacement for the existing 0.1.1 download.

A subsequent [independent local qualification](docs/final-qualification-2026-10-15.md)
corrected a repository-selection race and extended candidate hashes to the desktop
executable. Its final source passed 54 Rust and 19 frontend tests. A disposable
v0.1.0 installation migrated from database schema 2 to 3 and retained its selected
repository, theme and graph queries after the new candidate was installed. The
new local installer is unsigned and unpublished; native screen-reader, scaling,
long-duration and stable-Windows qualification remain open.

On 2026-10-09, all 38 Rust tests and 17 frontend tests passed, along with frontend
lint, type checking and the production frontend build. Native app and installer
checks passed on 2026-10-08, including upgrade, uninstall/reinstall and preservation
of the database and settings. GitHub checks run on pushes and pull requests.

See [release notes](docs/release.md), [performance](docs/performance.md) and
[security](docs/security.md) for methods and limits.

## Known limits

Static relationships cannot resolve arbitrary runtime dispatch, reflection or
computed imports. Diagnostics and confidence labels expose unresolved cases.
Impact is reachability, not a runtime prediction or a test-coverage guarantee.
Git renames appear as added and removed symbols. Local evidence is not encrypted.

Windows builds are unsigned. Trusted signing requires a code-signing certificate.
Linux/macOS, long-running desktop sessions and maximum-size WebView frame times
have not been validated. Whole-graph resolution and snapshot persistence still
make large refreshes expensive; measured workloads are in the performance docs.
