# Release validation

The current local 0.1.1 candidate also includes the Workshop Grid interface update.
Frontend checks and the final native build were repeated on 2026-10-08. The
[redesign record](workshop-grid.md) and [native captures](screenshots/workshop-grid/README.md)
describe the actual views, themes and narrow-window review. This remains a local
unsigned candidate, with the same engine/storage contracts.

Validated locally on Windows 11 x64 on 2026-10-07. Tauri production/NSIS build
completed, then the unsigned current-user installer installed into
artifacts/installed. Installed desktop and bundled CLI launched. Installed files
include both executables, GPL LICENSE and THIRD-PARTY-NOTICES.txt; current-user
registration reports Threadmark 0.1.1. Existing WebView2 is required and installer
runtime downloads are disabled. No public hosting or release publication occurred.

The existing 0.1.0 installation upgraded to 0.1.1 with schema 2→3 migration and
repository/theme preservation. `scripts/verify-installer.ps1` then validated
current-user upgrade, uninstall and reinstall, registration, installed binary
payloads and unchanged user data on uninstall. Its repeatable test requires the
app to be closed and refuses installations outside this project's artifacts
folder. The final machine report records the repeated 0.1.1 lifecycle:
[installer lifecycle](installer-lifecycle-windows-0.1.1.json).

Native checks used actual indexed source and Git objects: Ctrl+K/Enter search,
node/edge evidence, impact, detected related tests, working-tree changes and
removed-call baseline evidence, unresolved diagnostics, fit/zoom, Shift+F10 node
actions, and watch start/update/stop. A source edit reparsed one file and reused
eight. The original source was restored. The configured editor opened forecast.ts
in Notepad with custom CLI arguments. VS Code positional opening remains unverified
on this host. Native JSON save was verified to omit edge snippets and free-form
diagnostic messages. These checks supplement, rather than replace, automated
engine and frontend tests.

[Screenshots](screenshots/README.md) show installed native screens; their underlying
repository contains TypeScript, JSX, Python and an actual local Git baseline.
Generated benchmark sources are measured separately; [performance](performance.md)
reports exact workloads and limits. Runtime CSP and dependency review found no
network plugin, remote asset, hosted font or telemetry client. No active source
upload or repository-code execution path is included. Offline behavior was reviewed
from the production bundle and implemented boundaries; machine connectivity was
not disabled for a separate network-disconnection test.

Run scripts/package-release.ps1 after production builds. It produces installer,
portable CLI, corresponding project source and SHA256 checksums under artifacts/release.
Publish corresponding source and dependency notices with GPL binary distribution.
Building requires dependency and NSIS-tool downloads initially; installed runtime
analysis is local. Trusted code signing, hosted CI execution, other OSes
and an external security audit are not claimed as validated. See STATUS.md.

## Upstream and unsigned candidates

Git origin is https://github.com/Rhythm-Sanghi/Threadmark.git. The manually
dispatched `.github/workflows/release.yml` builds, checks and uploads Windows
installer, CLI, corresponding source, notices and checksums. It does not publish
a GitHub Release. Commit/push the reviewed project before running it in GitHub;
the local definition has not yet been exercised on a hosted runner.

Without signing secrets it produces an explicitly unsigned development/release
candidate. Unsigned candidates are acceptable for current testing. Trusted public
Windows signing remains a release blocker until a real certificate is obtained.
No certificate thumbprint is invented or stored in source.

To enable signing later, configure repository secrets `SIGNING_PFX_BASE64` and
`SIGNING_PFX_PASSWORD`, and the optional variable `SIGNING_TIMESTAMP_URL` (HTTPS
RFC 3161 service). The runner imports the certificate into its ephemeral current
user store, derives its thumbprint, calls `scripts/sign-release.ps1`, validates
CLI/desktop/installer Authenticode signatures and removes the temporary PFX and
imported signing certificate. A certificate without a trusted chain, usable
private key, current validity or code-signing usage is rejected. Hardware-backed
providers can use the local script with an installed certificate and SDK SignTool.

Local signing: `./scripts/sign-release.ps1 -CertificateThumbprint $thumbprint
-TimestampUrl $timestampUrl`. Keep certificate files, passwords and keys outside
the repository. Local unsigned build: build the release CLI, run `npm run tauri
build` in apps/desktop, then run `scripts/package-release.ps1` at project root.
