# 0004 — Load bounded graph slices

Implemented. Rust serves directed neighborhoods and evidence paths. React receives
a bounded slice, rather than loading or traversing the complete graph. Desktop
neighborhood requests use 24 nodes and one to three hops. Core slice requests are
capped at 120 nodes. Limits are marked so an incomplete view cannot imply that
there are no further relationships. Navigation refocuses or expands on demand.
