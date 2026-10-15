use std::sync::atomic::AtomicBool;
use threadmark_core::Engine;

#[test]
fn cancelled_or_failed_refresh_preserves_the_exact_previous_snapshot() {
    use std::sync::atomic::Ordering;
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let engine = Engine::open(Some(storage.path())).unwrap();
    std::fs::write(root.path().join("work.ts"), "export function previous() {}").unwrap();
    engine
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    let before = serde_json::to_value(engine.snapshot(None).unwrap()).unwrap();
    std::fs::write(
        root.path().join("work.ts"),
        "export function replacement() {}",
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        engine.index_repository(root.path(), &cancel, |_| cancel
            .store(true, Ordering::Relaxed)),
        Err(threadmark_core::Error::Cancelled)
    ));
    assert_eq!(
        serde_json::to_value(engine.snapshot(None).unwrap()).unwrap(),
        before
    );
    assert!(
        engine
            .index_repository(
                &root.path().join("missing"),
                &AtomicBool::new(false),
                |_| {}
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(engine.snapshot(None).unwrap()).unwrap(),
        before
    );
    drop(engine);
    let reopened = Engine::open(Some(storage.path())).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.snapshot(None).unwrap()).unwrap(),
        before
    );
    let refreshed = reopened
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(
        refreshed
            .nodes
            .iter()
            .any(|node| node.name == "replacement")
    );
    assert!(!refreshed.nodes.iter().any(|node| node.name == "previous"));
}

#[test]
fn reused_cache_survives_reopening_and_cross_process_updates_invalidate_adjacency() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let reader = Engine::open(Some(storage.path())).unwrap();
    let writer = Engine::open(Some(storage.path())).unwrap();
    let cancel = AtomicBool::new(false);
    std::fs::write(root.path().join("work.ts"), "export function work() {}").unwrap();
    reader
        .index_repository(root.path(), &cancel, |_| {})
        .unwrap();
    assert_eq!(
        reader
            .query(|graph| Ok(graph.search("work", 10).len()))
            .unwrap(),
        2
    );
    writer
        .index_repository(root.path(), &cancel, |_| {})
        .unwrap();
    let reopened = Engine::open(Some(storage.path())).unwrap();
    assert_eq!(
        reopened
            .index_repository(root.path(), &cancel, |_| {})
            .unwrap()
            .progress
            .reused_files,
        1
    );
    std::fs::write(root.path().join("work.ts"), "export function updated() {}").unwrap();
    writer
        .index_repository(root.path(), &cancel, |_| {})
        .unwrap();
    assert_eq!(
        reader
            .query(|graph| Ok(graph.search("updated", 10).len()))
            .unwrap(),
        1
    );
    std::fs::write(other.path().join("other.ts"), "export function other() {}").unwrap();
    writer
        .index_repository(other.path(), &cancel, |_| {})
        .unwrap();
    assert_eq!(
        reader
            .query(|graph| Ok(graph.search("updated", 10).len()))
            .unwrap(),
        1
    );
    assert_eq!(
        reader.snapshot(None).unwrap().root,
        root.path().canonicalize().unwrap().to_string_lossy()
    );
}
