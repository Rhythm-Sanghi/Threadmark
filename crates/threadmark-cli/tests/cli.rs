use std::process::Command;

#[test]
fn index_queries_tests_export_and_missing_editor_use_real_storage() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("work.ts"),
        "export function work(){} export function run(){work();unknownExternal();}",
    )
    .unwrap();
    std::fs::write(
        root.path().join("work.test.ts"),
        "import {work} from './work'; test('works',()=>work());",
    )
    .unwrap();
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_threadmark"))
            .args(["--data-dir", data.path().to_str().unwrap(), "--json"])
            .args(arguments)
            .output()
            .unwrap()
    };
    assert!(
        run(&["index", root.path().to_str().unwrap()])
            .status
            .success()
    );
    let output = run(&["impact", "work"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let impact: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        impact["direct"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["node"]["name"] == "run")
    );
    let output = run(&["tests", "work"]);
    let tests: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        !tests["detected_related_tests"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = run(&["export"]);
    let export: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        export["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edge| edge["evidence"] == "")
    );
    assert!(!export["diagnostics"].as_array().unwrap().is_empty());
    assert!(
        export["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["message"] == "")
    );
    let output = run(&["export", "--include-evidence"]);
    let explicit: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        explicit["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"] != "")
    );
    let output = run(&["open", "work"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Configure"));
    let output = run(&["impact", "--against", "missing-ref"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Git comparison"));
}

#[test]
fn cli_json_and_settings_roundtrip() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_threadmark"))
        .args([
            "--data-dir",
            directory.path().to_str().unwrap(),
            "--json",
            "settings",
            "--theme",
            "dark",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["schema_version"], 1);
    let output = Command::new(env!("CARGO_BIN_EXE_threadmark"))
        .args([
            "--data-dir",
            directory.path().to_str().unwrap(),
            "status",
            "--quiet",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn invalid_arguments_exit_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_threadmark"))
        .args(["settings", "--theme", "invalid"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn storage_failure_exits_one_without_contaminating_stdout() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_threadmark"))
        .args([
            "--data-dir",
            file.path().to_str().unwrap(),
            "--json",
            "status",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Could not create data directory"));
}
