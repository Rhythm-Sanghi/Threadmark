use std::sync::atomic::AtomicBool;
use threadmark_core::Engine;

#[test]
fn invalid_json_and_empty_cached_files_are_reparsed_and_repaired() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let engine = Engine::open(Some(data.path())).unwrap();
    let cancel = AtomicBool::new(false);
    std::fs::write(root.path().join("work.ts"), "export function work() {}").unwrap();
    let snapshot = engine
        .index_repository(root.path(), &cancel, |_| {})
        .unwrap();
    let db = threadmark_db::Database::open(&data.path().join("threadmark.db")).unwrap();
    let adapter = threadmark_core::parsing::ADAPTER_VERSION;
    let (path, hash, json) = db.cached_files(&snapshot.root, adapter).unwrap().remove(0);
    let mut empty: serde_json::Value = serde_json::from_str(&json).unwrap();
    empty["nodes"] = serde_json::json!([]);
    for corrupt in [
        "{broken".to_string(),
        serde_json::to_string(&empty).unwrap(),
    ] {
        db.save_index(
            &snapshot.root,
            snapshot.indexed_at as i64,
            &serde_json::to_string(&snapshot).unwrap(),
            adapter,
            &[(path.clone(), hash.clone(), Some(corrupt))],
        )
        .unwrap();
        let repaired = engine
            .index_repository(root.path(), &cancel, |_| {})
            .unwrap();
        assert_eq!(repaired.progress.parsed_files, 1);
        assert_eq!(repaired.progress.reused_files, 0);
        assert!(repaired.nodes.iter().any(|node| node.name == "work"));
        assert_eq!(
            engine
                .index_repository(root.path(), &cancel, |_| {})
                .unwrap()
                .progress
                .reused_files,
            1
        );
    }
}

#[test]
fn incremental_edits_deletions_renames_and_config_match_a_clean_index() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let engine = Engine::open(Some(data.path())).unwrap();
    let write = |path: &str, source: &str| std::fs::write(root.path().join(path), source).unwrap();
    write("a.ts", "export function work() {} ");
    write("b.ts", "export function work() { return 2; }");
    write("barrel.ts", "export {work as aliased} from '@target';");
    write(
        "main.ts",
        "import {aliased} from './barrel'; export function run(){aliased();}",
    );
    write("work.py", "def work():\n    return 1\n");
    write(
        "use.py",
        "from work import work as alias\ndef run():\n    return alias()\n",
    );
    write("broken.ts", "export function broken( {");
    write("unsupported.rs", "fn unsupported() {}");
    write(
        "tsconfig.json",
        r#"{"compilerOptions":{"paths":{"@target":["./a.ts"]}}}"#,
    );
    let signature = |snapshot: threadmark_core::model::Snapshot| {
        let mut nodes: Vec<_> = snapshot
            .nodes
            .iter()
            .map(|node| serde_json::to_string(node).unwrap())
            .collect();
        let mut edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|edge| serde_json::to_string(edge).unwrap())
            .collect();
        let mut diagnostics: Vec<_> = snapshot
            .diagnostics
            .iter()
            .map(|d| serde_json::to_string(d).unwrap())
            .collect();
        nodes.sort();
        edges.sort();
        diagnostics.sort();
        (nodes, edges, diagnostics)
    };
    for step in 0..6 {
        match step {
            1 => write("a.ts", "export function work(){return 3;}"),
            2 => {
                std::fs::remove_file(root.path().join("a.ts")).unwrap();
            }
            3 => {
                std::fs::rename(root.path().join("b.ts"), root.path().join("renamed.ts")).unwrap();
            }
            4 => write(
                "tsconfig.json",
                r#"{"compilerOptions":{"paths":{"@target":["./renamed.ts"]}}}"#,
            ),
            5 => write("work.py", "def changed():\n    return 2\n"),
            _ => (),
        }
        let incremental = engine
            .index_repository(root.path(), &AtomicBool::new(false), |_| {})
            .unwrap();
        let clean_data = tempfile::tempdir().unwrap();
        let clean = Engine::open(Some(clean_data.path()))
            .unwrap()
            .index_repository(root.path(), &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(signature(incremental), signature(clean), "step {step}");
    }
}

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
