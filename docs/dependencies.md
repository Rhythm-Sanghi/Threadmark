# Dependencies

Versions are resolved in Cargo.lock and apps/desktop/package-lock.json.
Selection checked upstream on 2026-10-07:

- [Tauri 2.12.1](https://docs.rs/tauri/2.12.1/tauri/) / tauri-build 2.7.1:
  MIT OR Apache-2.0; system WebView2 on Windows. Tauri 3 alpha is excluded.
- [React 19](https://react.dev/versions): MIT; installed stable React 19.3.0.
- [Vite 8](https://vite.dev/guide/): MIT; Node 24.15 meets its supported range.
- [TypeScript](https://www.typescriptlang.org/download/): Apache-2.0;
  6.0.3 selected because typescript-eslint 8.71.1 supports versions below 6.1.
- [rusqlite 0.40.2](https://docs.rs/rusqlite/0.40.2/rusqlite/): MIT;
  bundled SQLite (public domain) avoids an external database installation.
- serde/serde_json, clap, thiserror, tracing: MIT OR Apache-2.0.
- directories: MIT OR Apache-2.0, platform data-directory discovery.

Frontend testing/linting/build tools are development dependencies, not runtime
services. `scripts/audit-licenses.ps1` creates inventories from Cargo metadata and
npm package manifests. Review generated inventories before adding dependencies
and before distributing a release. A license expression inventory is not a legal
opinion or a complete distribution notice bundle. GPLv3 is compatible with the
listed direct dependency licenses. No concrete incompatibility is currently known.

Initial transitive inventory includes MIT/Apache, BSD, ISC, Zlib, Unicode-3.0,
CC0, BlueOak and MPL-2.0 expressions, plus LLVM exceptions. MPL-2.0 Rust crates
include cssparser, cssparser-macros, dtoa-short, option-ext and selectors; the
frontend build uses lightningcss under MPL-2.0. Preserve their notices and source
availability obligations when distributing. Multi-license crates with LGPL
alternatives also offer MIT/Apache. Registry metadata has no unlicensed named
package in the initial inventory. Audit covers resolved packages across platforms,
including development tools, not just the Windows executable.

Analysis dependencies: Tree-sitter 0.27 (MIT) and JS 0.25 / TypeScript 0.23.2 /
Python 0.25 grammars (MIT); ignore (MIT/Unlicense), blake3 (CC0/Apache), json5
(MIT), git2 0.21 (MIT/Apache), notify 8.2 (CC0), rfd 0.17 (MIT), ctrlc 3.5
(MIT/Apache), Windows bindings (MIT/Apache). Vendored libgit2 and zlib retain their
bundled license notices. libgit2 network features are disabled. No parser, database
or framework service is purchased or contacted at runtime.

scripts/generate-notices.ps1 collects installed Rust/npm license texts into
THIRD-PARTY-NOTICES.txt for packaging. Inventories cover all resolved platforms and
development dependencies, broader than the Windows runtime. Keep the lockfiles,
notices and corresponding source available with distributed binaries. This build
has not been published remotely.
