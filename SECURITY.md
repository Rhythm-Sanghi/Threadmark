# Security

Threadmark treats repositories as untrusted static input. Indexing does not run
repository scripts, modules, tests, hooks or plugins. The implemented boundary,
resource budgets, editor launch and local data handling are documented in
[docs/security.md](docs/security.md). Automated fixtures cover malformed source,
script non-execution, Unicode, junction traversal and source-free default export.
This project has not had an external security audit.

Report vulnerabilities privately through
[GitHub private vulnerability reporting](https://github.com/Rhythm-Sanghi/Threadmark/security/advisories/new).
Private reporting is enabled for this repository. Avoid posting source code or
credentials in public vulnerability reports.
