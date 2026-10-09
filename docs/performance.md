# Performance

Measured 2026-10-07 on Windows 11 x64, Intel Core i7-1365U (12 logical processors),
16 GB RAM, Rust 1.98.1 optimized release build. No concurrent build ran during the
measurements. These are single-run generated fixtures, not representative claims
for every repository. Windows filesystem cache was not flushed; cold means a new
Threadmark database and parse cache, not a physically cold disk.

Reproduce with `cargo run --release -p threadmark-core --example benchmark --locked`.
Each TypeScript file has ten functions in a directed call chain, one exported
entry, and no imports. Sizes exercise discovery and graph persistence, but do not
represent expensive alias resolution, dense monorepos or Git comparisons.

## Version 0.1.1 measurements

The updated implementation avoids repeated symbol scans, moves reusable parsed
files out of the cache rather than cloning them, skips serialization/writes for
unchanged cache entries and reads graph revisions from a small separate table.
The same fixture, on the same host without a concurrent build, measured:

| Files / functions | Cold index | Unchanged refresh | One-file update | Peak working set |
| --- | ---: | ---: | ---: | ---: |
| 100 / 1,000 | 102 ms | 48 ms | 60 ms | 21 MiB |
| 1,000 / 10,000 | 673 ms | 307 ms | 383 ms | 86 MiB |
| 10,000 / 100,000 | 7.40 s | 3.52 s | 4.60 s | 731 MiB |

For 10,000 files, unchanged refresh improved about 43% and one-file update about
29% compared with 0.1.0 below. Memory and database size (319 MiB) remain essentially
unchanged. First adjacency load was 973 ms, warm search 42.6 ms, bounded slice
0.205 ms and impact 0.270 ms; these query measurements were slower in this run.
Revision checks now ensure cached graphs reflect updates from other connections.
Single-run timings include background OS variability and are not latency guarantees.
Raw results: [0.1.1 benchmark](benchmark-windows-0.1.1.json).

A release engine soak ran 120 index/cache/adjacency/impact cycles over 1,000 files
and 10,000 functions in 52.2 seconds. Each update after the first reused 999 parses;
node counts and eight-hop impact results remained correct. Sampled working set
ranged from 36.5 to 41.9 MiB; the last sample was 41.9 MiB. This bounded run does
not establish multi-hour desktop memory stability or WebView frame performance.
Reproduce with `cargo run --release -p threadmark-core --example soak --locked`.
Raw results: [0.1.1 engine soak](soak-windows-0.1.1.json).

## Version 0.1.0 baseline

| Source files / functions | Cold index | Unchanged refresh | One-file update | Peak working set | SQLite size |
| --- | ---: | ---: | ---: | ---: | ---: |
| 100 / 1,000 | 69 ms | 40 ms | 60 ms | 21 MiB | 3.2 MiB |
| 1,000 / 10,000 | 678 ms | 387 ms | 421 ms | 86 MiB | 31.9 MiB |
| 10,000 / 100,000 | 8.25 s | 6.14 s | 6.46 s | 730 MiB | 319 MiB |

| Source files | Load adjacency from SQLite | Warm search | Two-hop slice | Eight-hop impact |
| --- | ---: | ---: | ---: | ---: |
| 100 | 7.08 ms | 0.49 ms | 0.018 ms | 0.108 ms |
| 1,000 | 56.6 ms | 1.87 ms | 0.011 ms | 0.045 ms |
| 10,000 | 696 ms | 26.6 ms | 0.013 ms | 0.049 ms |

Search returns at most 100 results. Slice limit is 24, but this chain returns only
three nodes. Impact returns eight reachable nodes and correctly marks its depth
limit; the very small query times must not be generalized to broad traversals.
The first query also incurs the separately measured adjacency load. Peak working
set is the Windows process high-water mark, cumulative over all three workloads.
Each update parses one file and reuses every other file. SQLite sizes exclude
OS cache and temporary benchmark directories.

Whole-graph resolution and JSON snapshot serialization still run on every refresh.
Thus the 10,000-file update is not instant despite parse reuse. Normalized graph
storage and affected-subgraph resolution are future optimization work. Memory
limits cap graph growth, but hundreds of MiB remain possible near these sizes.

An additional denser fixture (100 functions per file) took 4.31 s cold and 2.52 s
for one-file update at 100,000 functions / 1,000 files; its peak was 681 MiB. Raw
results: [main workload](benchmark-windows.json) and
[dense workload](benchmark-symbol-dense-windows.json).

The installed native WebView displayed a real nine-node neighborhood within the
263 ms keyboard-input-to-screenshot observation. This includes input automation,
IPC and screenshot overhead, is one sample, and is **not** a WebView frame-time
profile. Map pan/zoom, evidence and keyboard navigation were reviewed on actual
engine results. Desktop views request at most 24 nodes. Maximum-size frame-time
and long-session memory profiling remain unmeasured.
See [native observation](native-map-observation.json) and screenshots.
