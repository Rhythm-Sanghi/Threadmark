CREATE TABLE repositories (
    root TEXT PRIMARY KEY NOT NULL,
    indexed_at INTEGER NOT NULL,
    snapshot TEXT NOT NULL
) STRICT;
CREATE TABLE parsed_files (
    root TEXT NOT NULL REFERENCES repositories(root) ON DELETE CASCADE,
    path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    adapter_version TEXT NOT NULL,
    parsed TEXT NOT NULL,
    PRIMARY KEY(root, path)
) STRICT;
