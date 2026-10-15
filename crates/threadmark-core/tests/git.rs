use std::sync::atomic::AtomicBool;
use threadmark_core::Engine;
#[test]
fn compares_local_objects_deleted_symbols_and_untracked_sources() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let repo = git2::Repository::init(root).unwrap();
    std::fs::write(
        root.join("work.ts"),
        "export function work(){return 1;}\nexport function caller(){return work();}",
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("work.ts")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "baseline", &tree, &[])
        .unwrap();
    let storage = tempfile::tempdir().unwrap();
    let engine = Engine::open(Some(storage.path())).unwrap();
    engine
        .index_repository(root, &AtomicBool::new(false), |_| {})
        .unwrap();
    // Git blobs use LF while a Windows checkout commonly uses CRLF.
    let unchanged = std::fs::read_to_string(root.join("work.ts")).unwrap();
    std::fs::write(root.join("work.ts"), unchanged.replace('\n', "\r\n")).unwrap();
    let comparison = engine.compare("HEAD").unwrap();
    assert!(comparison.changed_files.is_empty());
    assert_eq!(comparison.symbol_change_count, 0);
    std::fs::write(root.join("work.ts"), "export function caller(){return 2;}").unwrap();
    std::fs::write(root.join("new.ts"), "export function added(){}").unwrap();
    let comparison = engine.compare("HEAD").unwrap();
    assert!(
        comparison
            .symbol_changes
            .iter()
            .any(|change| change.kind == "removed" && change.node.name == "work")
    );
    assert!(
        comparison
            .symbol_changes
            .iter()
            .any(|change| change.kind == "added" && change.node.name == "added")
    );
    assert!(
        comparison
            .removed_impact
            .direct
            .iter()
            .any(|item| item.node.name == "caller")
    );
    assert!(
        comparison
            .removed_relationships
            .iter()
            .any(|edge| edge.kind == "CALLS")
    );
    assert!(engine.compare("nonexistent-baseline").is_err());
    let other = tempfile::tempdir().unwrap();
    std::fs::write(
        other.path().join("elsewhere.ts"),
        "export function elsewhere() {}",
    )
    .unwrap();
    Engine::open(Some(storage.path()))
        .unwrap()
        .index_repository(other.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(
        Engine::open(Some(storage.path()))
            .unwrap()
            .snapshot(None)
            .unwrap()
            .root,
        other.path().canonicalize().unwrap().to_string_lossy()
    );
    // A comparison refreshes its own repository, but must not switch another
    // instance's selected repository back after the user has moved on.
    engine.compare("HEAD").unwrap();
    assert_eq!(
        Engine::open(Some(storage.path()))
            .unwrap()
            .snapshot(None)
            .unwrap()
            .root,
        other.path().canonicalize().unwrap().to_string_lossy()
    );
    assert_eq!(
        std::fs::read_to_string(root.join("work.ts")).unwrap(),
        "export function caller(){return 2;}"
    );
}
