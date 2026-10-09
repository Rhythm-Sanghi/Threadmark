# 0002 — Deterministic symbol identity

Implemented. BLAKE3 hashes length-prefixed repository root, relative file path,
language, qualified symbol name and kind. Source line numbers do not participate;
moving a declaration within its file preserves its ID. Renaming or moving a file
produces added/removed IDs. Duplicate declarations are diagnosed and excluded
from confident name resolution. Indexing and watcher fixtures check identity.

Content fingerprints normalize CRLF to LF for local Git comparison; byte-sensitive
file hashes still invalidate parse caches and preserve accurate source spans.
