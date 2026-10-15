# Windows release notes

Version 0.1.1 is an unsigned Windows release candidate. Source is published at
[Rhythm-Sanghi/Threadmark](https://github.com/Rhythm-Sanghi/Threadmark).
Download the installer, portable CLI, source and checksums from the
[0.1.1 prerelease](https://github.com/Rhythm-Sanghi/Threadmark/releases/tag/v0.1.1).

## Local checks

The Windows 11 x64 build and NSIS installer were checked on 2026-10-07–08.
The installer includes the desktop app, CLI, GPL license and dependency notices.
It installs for the current user and requires an existing WebView2 runtime.

Upgrade from 0.1.0 migrated the database from schema 2 to 3 and preserved the
selected repository and theme. The later isolated
[qualification](final-qualification-2026-10-15.md) repeats that genuine
previous-version upgrade for the new local candidate. `scripts/verify-installer.ps1`
checks the lifecycle from whichever version is currently installed, including
uninstall/reinstall, registration, binary payloads and preservation of user data.
It requires the app to be closed and refuses installations outside
`artifacts/installed`. The historical
[lifecycle report](installer-lifecycle-windows-0.1.1.json) contains earlier results.

Native checks covered search, map navigation, source and relationship evidence,
impact, related tests, Git changes, removed-symbol evidence, diagnostics, file
watching, metadata export, light/dark themes and narrow windows. Notepad opening
was checked; VS Code positional navigation has not been verified on this host.
The development console showed no application exceptions.

On 2026-10-09, 38 Rust tests and 17 frontend tests passed, along with frontend
lint, type checking and the production frontend build. See [project status](../STATUS.md)
and [performance](performance.md) for known limits and measured workloads.

## Build and package

Use PowerShell 7 and the prerequisites in the README. Install locked frontend
dependencies with `npm ci` in apps/desktop. Generate dependency notices with
`scripts/generate-notices.ps1`, review them and commit any changes before building.
From a clean checkout without untracked source files, run:

```powershell
./scripts/build-candidate.ps1
./scripts/package-release.ps1 -OutputDirectory artifacts/candidate-2026-10-15
```

The builder compiles CLI and native installer, then records their hashes and the
source commit in ignored artifacts/release-build.json. Packaging rejects changed
binaries, another commit, tracked edits and nonempty output directories. Choose
a new empty output directory for each candidate; old release artifacts are never
silently mixed into a new checksum list. The receipt records a local build, not a
signed supply-chain attestation or independent verification.

Source ZIPs use `git archive` for a resolved commit, preserving committed dotfiles
and excluding all untracked files and working-tree edits. SOURCE-MANIFEST.json
records the commit and Git blob inventory; SHA256SUMS.txt covers the output files.
For source alone, use `scripts/package-release.ps1 -SourceOnly -SourceRef v0.1.1
-OutputDirectory artifacts/source-check`. The source ZIP is repeatable for the
same commit and Git version; installer reproducibility is not established.
Run `scripts/test-package-release.ps1` to verify provenance boundaries in an
isolated temporary Git fixture. Include source and notices with GPL binaries.

The Checks workflow runs on pushes and pull requests. The manually dispatched
Windows release candidate workflow builds and uploads installer, CLI, source,
notices and checksums as workflow artifacts; it does not create a GitHub Release.

## Signing

Without signing secrets, builds remain unsigned. Trusted Windows signing requires
a real code-signing certificate. Keep certificate files and passwords outside Git.

To sign in CI, configure `SIGNING_PFX_BASE64` and `SIGNING_PFX_PASSWORD` secrets,
and optionally `SIGNING_TIMESTAMP_URL` with an HTTPS RFC 3161 service. The workflow
imports the certificate temporarily, derives its thumbprint, validates signatures
and removes the temporary certificate and PFX. No thumbprint is hard-coded.

For local signing and packaging provenance, use `scripts/build-candidate.ps1
-SigningThumbprint $thumbprint -TimestampUrl $timestampUrl`, then the packaging
command above. The underlying sign-release.ps1 can also sign a standalone build;
it no longer packages automatically. Use an installed certificate and SDK SignTool.
Hardware-backed certificates can use the local script.

Linux/macOS, multi-hour desktop sessions and maximum-size WebView frame times
remain unvalidated. The project has not had an external security audit.
