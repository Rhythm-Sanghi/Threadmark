use std::sync::atomic::{AtomicBool, Ordering};
use threadmark_core::{
    Engine, Error,
    parsing::{JavaScript, LanguageAdapter},
};

#[test]
fn extracts_ast_symbols_imports_exports_and_exact_spans() {
    let source = "import type { User as Person } from './user';\nexport function hello(user: Person) { return user; }\n";
    let parsed = JavaScript.parse("repo", "src/main.ts", source).unwrap();
    let hello = parsed
        .nodes
        .iter()
        .find(|node| node.name == "hello")
        .unwrap();
    assert_eq!(hello.kind, "function");
    assert_eq!(hello.span.line, 2);
    assert!(hello.exported);
    assert!(source[hello.span.start_byte..hello.span.end_byte].starts_with("function hello"));
    assert_eq!(
        parsed.imports[0].bindings,
        [("Person".into(), "User".into())]
    );
    assert!(parsed.imports[0].type_only);
    let shifted = JavaScript
        .parse("repo", "src/main.ts", &format!("\n{source}"))
        .unwrap();
    assert_eq!(
        hello.id,
        shifted
            .nodes
            .iter()
            .find(|node| node.name == "hello")
            .unwrap()
            .id
    );
}

#[test]
fn ignore_limits_broken_code_and_incremental_cache() {
    let repository = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    std::fs::write(repository.path().join(".gitignore"), "ignored.ts\n").unwrap();
    std::fs::write(repository.path().join(".threadmarkignore"), "custom.ts\n").unwrap();
    for file in ["ignored.ts", "custom.ts", "valid.ts"] {
        std::fs::write(repository.path().join(file), "export function work() {}\n").unwrap();
    }
    std::fs::create_dir(repository.path().join("node_modules")).unwrap();
    std::fs::write(repository.path().join("node_modules/bad.ts"), "not source").unwrap();
    std::fs::write(repository.path().join("broken.ts"), "function broken( {").unwrap();
    std::fs::write(repository.path().join("invalid.ts"), [255, 254]).unwrap();
    std::fs::write(
        repository.path().join("huge.ts"),
        vec![b' '; 2 * 1024 * 1024 + 1],
    )
    .unwrap();
    let engine = Engine::open(Some(storage.path())).unwrap();
    let snapshot = engine
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(snapshot.progress.parsed_files, 2);
    assert!(
        snapshot
            .nodes
            .iter()
            .all(|node| !node.path.contains("ignored")
                && !node.path.contains("custom")
                && !node.path.contains("node_modules"))
    );
    assert!(snapshot.diagnostics.iter().any(|d| d.code == "parse-error"));
    assert_eq!(
        snapshot
            .diagnostics
            .iter()
            .filter(|d| d.code == "read-error")
            .count(),
        2
    );
    let snapshot = engine
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(snapshot.progress.reused_files, 2);
    let cancelled = AtomicBool::new(false);
    assert!(matches!(
        engine.index_repository(repository.path(), &cancelled, |_| cancelled
            .store(true, Ordering::Relaxed)),
        Err(Error::Cancelled)
    ));
    assert_eq!(
        engine.snapshot(None).unwrap().nodes.len(),
        snapshot.nodes.len()
    );
}

#[test]
fn malformed_and_dynamic_imports_are_explicit() {
    let parsed = JavaScript
        .parse(
            "repo",
            "main.js",
            "const x = import(target); handlers[event.type]();",
        )
        .unwrap();
    assert!(parsed.imports.is_empty());
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == "dynamic-import")
    );
    assert!(parsed.diagnostics.iter().any(|d| d.code == "dynamic-call"));
}
