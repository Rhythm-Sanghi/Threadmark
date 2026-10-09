# Graph model

Nodes carry stable ID, name, qualified name, kind, relative path, language, source
span, exported/test flags and a declaration fingerprint. IDs are BLAKE3 hashes of
length-prefixed repository/path/language/qualified-name/kind fields. Blank lines do
not change identity. A rename changes path/identity and appears as added/removed.
Duplicate declarations retain one navigable node and an ambiguity diagnostic;
relationships requiring ambiguous declarations are excluded.

Relationships are directed source -> dependency. CONTAINS and EXPORTS are structural;
IMPORTS connect files. CALLS, REFERENCES, USES_TYPE, TESTS, ROUTES_TO and READS_CONFIG
connect declarations using static evidence. Each edge retains confidence, source
path/span, up to 500 evidence characters and adapter. EXACT is direct syntax;
RESOLVED means an unambiguous target through supported resolution. No runtime
binding guarantee is implied. This release does not manufacture inferred edges;
the impact engine supports INFERRED paths when future adapters can supply them.

Span lines/columns are one-based, with UTF-8 byte offsets. Stable edge IDs include
source/target/kind/confidence and span; Git comparison deliberately compares the
semantic tuple instead, so line shifts do not count as relationship changes.
Configuration-key identity is shared by key name across source files; .env values
are not part of the model.

Rust adjacency indexes support search, bounded slices, shortest directed paths,
reverse impact and strongly connected local-import components. Map slices default
to 24 nodes, maximum query depth 3; path traversal visits at most 10,000 nodes and
32 hops. Impact has separately documented limits. Circular import diagnostics mean
mutual reachability, with the real edges available through threadmark cycles.
Clients receive versioned DTOs, not database connections or arbitrary SQL.
