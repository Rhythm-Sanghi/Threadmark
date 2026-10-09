use std::sync::atomic::AtomicBool;
use threadmark_core::Engine;

fn index(files: &[(&str, &str)]) -> threadmark_core::model::Snapshot {
    let repository = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    for (path, source) in files {
        let path = repository.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    Engine::open(Some(storage.path()))
        .unwrap()
        .index_repository(repository.path(), &AtomicBool::new(false), |_| {})
        .unwrap()
}

fn relationship(
    snapshot: &threadmark_core::model::Snapshot,
    from: &str,
    to: &str,
    kind: &str,
) -> bool {
    snapshot.edges.iter().any(|edge| {
        edge.kind == kind
            && snapshot
                .nodes
                .iter()
                .any(|n| n.id == edge.from && n.name == from)
            && snapshot
                .nodes
                .iter()
                .any(|n| n.id == edge.to && n.name == to)
    })
}

#[test]
fn imports_aliases_reexports_calls_and_types() {
    let snapshot = index(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@lib/*":["src/*"]}}}"#,
        ),
        (
            "src/math.ts",
            "export function double(x: number) { return x*2; } export interface Input { value: number }",
        ),
        (
            "src/barrel.ts",
            "export { double as twice, Input } from './math';",
        ),
        (
            "src/main.ts",
            "import { twice, type Input } from '@lib/barrel'; export function run(x: Input) { return twice(x.value); }",
        ),
    ]);
    assert!(relationship(&snapshot, "run", "double", "CALLS"));
    assert!(relationship(&snapshot, "run", "Input", "USES_TYPE"));
    let call = snapshot
        .edges
        .iter()
        .find(|edge| edge.kind == "CALLS")
        .unwrap();
    assert_eq!(call.evidence, "twice(x.value)");
    assert_eq!(call.confidence, "RESOLVED");
    assert!(
        !snapshot
            .diagnostics
            .iter()
            .any(|d| d.code == "unresolved-import")
    );
}

#[test]
fn workspace_exports_and_circular_barrels() {
    let snapshot = index(&[
        (
            "packages/lib/package.json",
            r#"{"name":"@demo/lib","exports":{".":"./src/index.ts"}}"#,
        ),
        ("packages/lib/src/index.ts", "export * from './a';"),
        (
            "packages/lib/src/a.ts",
            "export * from './index'; export function work() {}",
        ),
        (
            "apps/main.ts",
            "import { work } from '@demo/lib'; function run() { work(); }",
        ),
    ]);
    assert!(relationship(&snapshot, "run", "work", "CALLS"));
}

#[test]
fn unresolved_imports_do_not_invent_edges() {
    let snapshot = index(&[(
        "main.ts",
        "import { work } from './missing'; function run() { work(); } import(target);",
    )]);
    assert!(!snapshot.edges.iter().any(|edge| edge.kind == "CALLS"));
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|d| d.code == "unresolved-import")
    );
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|d| d.code == "dynamic-import")
    );
}

#[test]
fn parameter_shadowing_and_overloads_never_resolve_to_global_names() {
    let snapshot = index(&[(
        "main.ts",
        "function work() {} function run(work: () => void) { work(); } function twice(x: string): string; function twice(x: number): number; function twice(x: any) { return x; } function invoke() { twice(2); }",
    )]);
    assert!(!relationship(&snapshot, "run", "work", "CALLS"));
    assert!(!relationship(&snapshot, "invoke", "twice", "CALLS"));
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|d| d.code == "duplicate-symbol")
    );
}

#[test]
fn commonjs_namespace_and_default_export() {
    let snapshot = index(&[
        (
            "math.cjs",
            "function double(x) { return x*2; } module.exports = { double };",
        ),
        (
            "main.cjs",
            "const math = require('./math.cjs'); function run() { math.double(2); }",
        ),
        ("default.ts", "export default () => 42;"),
        (
            "consumer.ts",
            "import value from './default'; function read() { return value(); }",
        ),
    ]);
    assert!(relationship(&snapshot, "run", "double", "CALLS"));
    assert!(relationship(&snapshot, "read", "default", "CALLS"));
}

#[test]
fn relative_config_inheritance_preserves_base_origin_and_child_overrides() {
    let snapshot = index(&[
        (
            "config/base.json",
            r#"{"compilerOptions":{"baseUrl":"../src","paths":{"@/*":["*"]}}}"#,
        ),
        ("apps/tsconfig.json", r#"{"extends":"../config/base"}"#),
        ("src/work.ts", "export function work() {}"),
        (
            "apps/main.ts",
            "import {work} from '@/work'; function run(){work();}",
        ),
    ]);
    assert!(relationship(&snapshot, "run", "work", "CALLS"));
    assert!(
        !snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "config-extends")
    );
    let snapshot = index(&[
        (
            "config/base.json",
            r#"{"compilerOptions":{"baseUrl":"../old","paths":{"@/*":["*"]}}}"#,
        ),
        (
            "apps/tsconfig.json",
            r#"{"extends":"../config/base.json","compilerOptions":{"baseUrl":"../src"}}"#,
        ),
        ("src/work.ts", "export function work() {}"),
        (
            "apps/main.ts",
            "import {work} from '@/work'; function run(){work();}",
        ),
    ]);
    assert!(relationship(&snapshot, "run", "work", "CALLS"));
}

#[test]
fn inherited_config_cycles_and_outside_paths_remain_diagnostics() {
    let snapshot = index(&[
        (
            "tsconfig.json",
            r#"{"extends":["./loop.json","../outside.json","external-package"]}"#,
        ),
        ("loop.json", r#"{"extends":"./tsconfig.json"}"#),
        ("work.ts", "function work() {}"),
    ]);
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("cycle"))
    );
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("outside"))
    );
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("external-package"))
    );
}

#[test]
fn reassigned_functions_and_imports_do_not_create_confident_calls() {
    let snapshot = index(&[
        (
            "work.ts",
            "export function work() {} work = runtimeHandler;",
        ),
        (
            "main.ts",
            "import {work} from './work'; function run(){work();}",
        ),
        (
            "local.ts",
            "function stable() {} function invoke(){stable();} function replace(){stable = unknown;} function untouched() {} function other(){untouched();}",
        ),
    ]);
    assert!(!relationship(&snapshot, "run", "work", "CALLS"));
    assert!(!relationship(&snapshot, "invoke", "stable", "CALLS"));
    assert!(relationship(&snapshot, "other", "untouched", "CALLS"));
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "reassigned-binding")
    );
}
