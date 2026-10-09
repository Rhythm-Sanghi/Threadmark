# Security

Threadmark treats repositories as untrusted static input. Indexing does not run
repository scripts, modules, tests, hooks or plugins. The implemented boundary,
resource budgets, editor launch and local data handling are documented in
[docs/security.md](docs/security.md). Automated fixtures cover malformed source,
script non-execution, Unicode, junction traversal and source-free default export.
This project has not had an external security audit.

The configured upstream is https://github.com/Rhythm-Sanghi/Threadmark.
Its owner must enable GitHub private vulnerability reporting before public
distribution; hosted configuration has not been verified from this workspace.
Once enabled, report privately through the repository's Security tab.
Avoid posting source code or credentials in public vulnerability reports.
