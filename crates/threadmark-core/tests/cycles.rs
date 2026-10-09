use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, graph::Graph};
#[test]
fn circular_import_components_have_actual_edges_and_no_acyclic_nodes() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("a.ts", "import './b';"),
        ("b.ts", "import './c';"),
        ("c.ts", "import './a';"),
        ("outside.ts", "import './a';"),
    ] {
        std::fs::write(root.path().join(path), source).unwrap();
    }
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "circular-dependency")
    );
    let cycles = Graph::new(snapshot).cycles();
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].nodes.len(), 3);
    assert_eq!(cycles[0].edges.len(), 3);
    assert!(!cycles[0].nodes.iter().any(|node| node.path == "outside.ts"));
}
