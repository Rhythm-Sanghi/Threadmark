use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, graph::Graph};
#[test]
fn test_callbacks_own_resolved_calls_and_related_tests_keep_paths() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("work.ts"),
        "export function work(){return 1;}",
    )
    .unwrap();
    std::fs::write(root.path().join("work.test.ts"),"import {test} from 'vitest'; import {work} from './work'; test('works',()=>{work();}); test('unrelated',()=>{unknown();});").unwrap();
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .filter(|node| node.kind == "test")
            .count(),
        2
    );
    let graph = Graph::new(snapshot);
    let impact = graph.impact(&["work".into()], 8, &[]).unwrap();
    assert!(
        impact
            .related_tests
            .iter()
            .any(|item| item.node.name == "works")
    );
    assert!(
        !impact
            .related_tests
            .iter()
            .any(|item| item.node.name == "unrelated")
    );
    let test = graph.select("works").unwrap();
    assert!(
        graph
            .snapshot
            .edges
            .iter()
            .any(|edge| edge.from == test.id && edge.kind == "TESTS" && edge.evidence == "work()")
    );
}
