use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, graph::Graph};
#[test]
fn python_modules_aliases_methods_tests_and_parameter_shadowing() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("pkg")).unwrap();
    std::fs::write(
        root.path().join("pkg/__init__.py"),
        "from .work import work\n",
    )
    .unwrap();
    std::fs::write(root.path().join("pkg/work.py"),"def work():\n    return 1\n\nclass Worker:\n    def run(self):\n        return self.normalize()\n    def normalize(self):\n        return work()\n").unwrap();
    std::fs::write(root.path().join("test_work.py"),"from pkg import work as run\nimport pkg.work as module\ndef test_work():\n    run()\n    module.work()\ndef shadowed(run):\n    run()\n").unwrap();
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    let graph = Graph::new(snapshot);
    let work = graph.select("work").unwrap();
    let test = graph.select("test_work").unwrap();
    assert!(test.test);
    assert!(
        graph
            .snapshot
            .edges
            .iter()
            .any(|edge| edge.from == test.id && edge.to == work.id && edge.kind == "CALLS")
    );
    let method = graph.select("Worker.run").unwrap();
    let normalize = graph.select("Worker.normalize").unwrap();
    assert!(
        graph
            .snapshot
            .edges
            .iter()
            .any(|edge| edge.from == method.id && edge.to == normalize.id && edge.kind == "CALLS")
    );
    let shadow = graph.select("shadowed").unwrap();
    assert!(
        !graph
            .snapshot
            .edges
            .iter()
            .any(|edge| edge.from == shadow.id && edge.kind == "CALLS")
    );
    assert!(
        graph
            .impact(std::slice::from_ref(&work.id), 8, &[])
            .unwrap()
            .related_tests
            .iter()
            .any(|item| item.node.id == test.id)
    );
}
#[test]
fn python_does_not_execute_top_level_code_and_bad_syntax_is_visible() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("bad.py"),
        "open('sentinel','w').write('executed')\ndef broken(:\n pass\n",
    )
    .unwrap();
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    assert!(!root.path().join("sentinel").exists());
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "parse-error")
    );
}
