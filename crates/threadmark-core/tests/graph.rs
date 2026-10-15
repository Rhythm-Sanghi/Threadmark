use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, Error, indexing};

#[test]
fn bounded_slices_search_paths_and_source_preview() {
    let repository = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    std::fs::write(
        repository.path().join("math.ts"),
        "export function double(n: number) { return n*2; }",
    )
    .unwrap();
    std::fs::write(
        repository.path().join("run.ts"),
        "import { double } from './math'; export function run() { return double(2); }",
    )
    .unwrap();
    let engine = Engine::open(Some(storage.path())).unwrap();
    engine
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    engine
        .query(|graph| {
            assert_eq!(graph.search("double", 10)[0].name, "double");
            let slice = graph.slice("double", "dependents", 1, 1, &[])?;
            assert_eq!(slice.nodes.len(), 1);
            assert!(slice.truncated);
            let path = graph.find_path("run", "double", 8)?;
            assert_eq!(path.nodes.len(), 2);
            assert_eq!(path.edges[0].kind, "CALLS");
            assert_eq!(graph.tree("").len(), 2);
            assert!(matches!(graph.select("missing"), Err(Error::Analysis(_))));
            Ok(())
        })
        .unwrap();
    assert!(engine.excerpt("double").unwrap().lines[0].contains("export function"));
    std::fs::remove_file(repository.path().join("math.ts")).unwrap();
    assert!(engine.excerpt("double").is_err());
    assert!(indexing::safe_read(repository.path(), "../escape", 10).is_err());
}

#[test]
fn package_scripts_are_never_executed_and_unicode_names_work() {
    let repository = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    std::fs::write(
        repository.path().join("package.json"),
        r#"{"scripts":{"preinstall":"echo owned > SENTINEL","test":"echo owned > SENTINEL"}}"#,
    )
    .unwrap();
    std::fs::write(
        repository.path().join("café.ts"),
        "export function café() { return 1; }",
    )
    .unwrap();
    let snapshot = Engine::open(Some(storage.path()))
        .unwrap()
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(snapshot.nodes.iter().any(|node| node.name == "café"));
    assert!(!repository.path().join("SENTINEL").exists());
}

#[cfg(windows)]
#[test]
fn junctions_cannot_escape_or_recurse() {
    let repository = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.ts"), "export const secret = 1;").unwrap();
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(repository.path().join("outside"))
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(repository.path().join("loop"))
        .arg(repository.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    let snapshot = Engine::open(Some(storage.path()))
        .unwrap()
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(snapshot.nodes.is_empty());
    assert!(indexing::safe_read(repository.path(), "outside/secret.ts", 1024).is_err());
}

#[cfg(windows)]
#[test]
fn opened_handle_check_retries_long_paths_without_relaxing_the_boundary() {
    let repository = tempfile::tempdir().unwrap();
    let root = repository.path().canonicalize().unwrap();
    let relative = format!(
        "{}/work.ts",
        std::iter::repeat_n("long_directory_segment", 26)
            .collect::<Vec<_>>()
            .join("/")
    );
    let path = root.join(&relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "export function long_path() {}").unwrap();
    assert!(path.as_os_str().len() > 512);
    assert_eq!(
        indexing::safe_read(&root, &relative, 1024).unwrap(),
        "export function long_path() {}"
    );
    assert!(indexing::safe_read(&root, &relative, 2).is_err());
}

#[cfg(windows)]
#[test]
fn unreadable_source_is_diagnosed_and_recovers_after_handle_release() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("blocked.ts"),
        "export function blocked() {}",
    )
    .unwrap();
    std::fs::write(root.path().join("valid.ts"), "export function valid() {}").unwrap();
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(root.path().join("blocked.ts"))
        .unwrap();
    let engine = Engine::open(Some(data.path())).unwrap();
    let partial = engine
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(
        partial
            .diagnostics
            .iter()
            .any(|d| d.code == "read-error" && d.path == "blocked.ts")
    );
    assert!(partial.nodes.iter().any(|n| n.name == "valid"));
    assert!(!partial.nodes.iter().any(|n| n.name == "blocked"));
    drop(locked);
    let complete = engine
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(complete.nodes.iter().any(|n| n.name == "blocked"));
    assert!(!complete.diagnostics.iter().any(|d| d.code == "read-error"));
}
