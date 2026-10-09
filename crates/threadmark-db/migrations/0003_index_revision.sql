CREATE TABLE index_revisions (
    root TEXT PRIMARY KEY NOT NULL REFERENCES repositories(root) ON DELETE CASCADE,
    revision INTEGER NOT NULL
) STRICT;
INSERT INTO index_revisions(root,revision) SELECT root,1 FROM repositories;
