use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, graph::Graph};
#[test]
fn environment_key_relationships_share_identity_without_reading_values() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join(".env"),
        "DATABASE_URL=private-secret-value",
    )
    .unwrap();
    std::fs::write(
        root.path().join("db.ts"),
        "export function connect(){return process.env.DATABASE_URL;}",
    )
    .unwrap();
    std::fs::write(
        root.path().join("job.ts"),
        "export function start(){return process.env.DATABASE_URL;}",
    )
    .unwrap();
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("private-secret-value")
    );
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .filter(|node| node.kind == "configuration_key")
            .count(),
        1
    );
    let graph = Graph::new(snapshot);
    let impact = graph
        .impact(&["config:DATABASE_URL".into()], 8, &["READS_CONFIG".into()])
        .unwrap();
    assert_eq!(impact.direct.len(), 2);
    assert!(
        impact
            .direct
            .iter()
            .all(|item| item.evidence_path[0].evidence == "process.env.DATABASE_URL")
    );
}
