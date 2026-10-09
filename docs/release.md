# Windows release notes

Version 0.1.1 is an unsigned Windows release candidate. Source is published at
[Rhythm-Sanghi/Threadmark](https://github.com/Rhythm-Sanghi/Threadmark).
A downloadable GitHub Release is not available yet.

## Local checks

The Windows 11 x64 build and NSIS installer were checked on 2026-10-07–08.
The installer includes the desktop app, CLI, GPL license and dependency notices.
It installs for the current user and requires an existing WebView2 runtime.

Upgrade from 0.1.0 migrated the database from schema 2 to 3 and preserved the
selected repository and theme. `scripts/verify-installer.ps1` checks upgrade,
uninstall/reinstall, registration, binary payloads and preservation of user data.
It requires the app to be closed and refuses installations outside artifacts/installed.
The [lifecycle report](installer-lifecycle-windows-0.1.1.json) contains the results.

Native checks covered search, map navigation, source and relationship evidence,
impact, related tests, Git changes, removed-symbol evidence, diagnostics, file
watching, metadata export, light/dark themes and narrow windows. Notepad opening
was checked; VS Code positional navigation has not been verified on this host.
The development console showed no application exceptions.

On 2026-10-09, 38 Rust tests and 17 frontend tests passed, along with frontend
lint, type checking and the production frontend build. See [project status](../STATUS.md)
and [performance](performance.md) for known limits and measured workloads.

## Build and package

Use PowerShell 7 and the prerequisites in the README. Build the release CLI,
then run `npm run tauri build` in apps/desktop. From the repository root, run
`scripts/generate-notices.ps1` and `scripts/package-release.ps1`. Archives and
SHA256 checksums are written to artifacts/release. Include corresponding source
and dependency notices when distributing GPL binaries.

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

For local signing, use `scripts/sign-release.ps1 -CertificateThumbprint $thumbprint
-TimestampUrl $timestampUrl` with an installed certificate and SDK SignTool.
Hardware-backed certificates can use the local script.

Linux/macOS, multi-hour desktop sessions and maximum-size WebView frame times
remain unvalidated. The project has not had an external security audit.
