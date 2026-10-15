//! Repeatable mixed-dependency qualification workload. Run in release mode.
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{atomic::AtomicBool, mpsc},
    time::{Duration, Instant},
};
use threadmark_core::{Engine, model::Snapshot, watch::WatchHandle};

const WARMUPS: usize = 3;
const MEASURED: usize = 5;
const MODULES: usize = 300;

fn peak_working_set_bytes() -> usize {
    #[cfg(windows)]
    {
        let mut counters =
            windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS::default();
        let size = std::mem::size_of_val(&counters) as u32;
        let success = unsafe {
            windows_sys::Win32::System::ProcessStatus::GetProcessMemoryInfo(
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                &mut counters,
                size,
            )
        };
        if success == 0 {
            0
        } else {
            counters.PeakWorkingSetSize
        }
    }
    #[cfg(not(windows))]
    {
        0
    }
}

fn index(engine: &Engine, root: &Path) -> Snapshot {
    engine
        .index_repository(root, &AtomicBool::new(false), |_| {})
        .expect("index fixture")
}

fn sample(
    values: &mut BTreeMap<String, Vec<f64>>,
    name: &str,
    iteration: usize,
    operation: impl FnOnce() -> Snapshot,
) {
    let started = Instant::now();
    let snapshot = operation();
    assert!(snapshot.nodes.len() >= MODULES);
    if iteration >= WARMUPS {
        values
            .entry(name.into())
            .or_default()
            .push(started.elapsed().as_secs_f64() * 1000.);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let data = tempfile::tempdir()?;
    let modules: Vec<String> = (0..MODULES)
        .map(|i| {
            if i == 0 {
                "export function value() { return 1; }\n".into()
            } else {
                format!(
                    "import {{ value as previous }} from './module{}'; export function value() {{ return previous() + {}; }}\n",
                    i - 1,
                    i % 7
                )
            }
        })
        .collect();
    for (i, source) in modules.iter().enumerate() {
        fs::write(root.path().join(format!("module{i}.ts")), source)?;
    }
    fs::write(
        root.path().join("barrel.ts"),
        "export { value as rootValue } from './module0';\n",
    )?;
    fs::write(
        root.path().join("consumer.ts"),
        "import { rootValue } from './barrel'; export function consume() { return rootValue(); }\n",
    )?;
    for i in 0..10 {
        fs::write(
            root.path().join(format!("module{i}.test.ts")),
            format!("import {{ value }} from './module{i}'; test('value', () => value());\n"),
        )?;
    }
    fs::write(
        root.path().join("tsconfig.json"),
        "{\"compilerOptions\":{}}",
    )?;
    let engine = Engine::open(Some(data.path()))?;
    let mut values: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for i in 0..WARMUPS + MEASURED {
        let cold = tempfile::tempdir()?;
        let fresh = Engine::open(Some(cold.path()))?;
        sample(&mut values, "cold", i, || index(&fresh, root.path()));
    }
    index(&engine, root.path());
    for i in 0..WARMUPS + MEASURED {
        sample(&mut values, "unchanged", i, || index(&engine, root.path()));
    }
    for i in 0..WARMUPS + MEASURED {
        fs::write(
            root.path().join("module0.ts"),
            format!("export function value() {{ return {}; }}\n", i + 2),
        )?;
        sample(&mut values, "one_file_update", i, || {
            index(&engine, root.path())
        });
    }
    for i in 0..WARMUPS + MEASURED {
        fs::remove_file(root.path().join(format!("module{}.ts", MODULES - 1)))?;
        sample(&mut values, "file_deletion", i, || {
            index(&engine, root.path())
        });
        fs::write(
            root.path().join(format!("module{}.ts", MODULES - 1)),
            &modules[MODULES - 1],
        )?;
        index(&engine, root.path());
    }
    for i in 0..WARMUPS + MEASURED {
        fs::write(
            root.path().join("barrel.ts"),
            format!(
                "export {{ value as rootValue }} from './module{}';\n",
                i % 2
            ),
        )?;
        sample(&mut values, "reexport_change", i, || {
            index(&engine, root.path())
        });
    }
    for i in 0..WARMUPS + MEASURED {
        fs::write(
            root.path().join("consumer.ts"),
            format!(
                "import {{ value }} from './module{}'; export function consume() {{ return value(); }}\n",
                i % 2
            ),
        )?;
        sample(&mut values, "import_change", i, || {
            index(&engine, root.path())
        });
    }
    for i in 0..WARMUPS + MEASURED {
        fs::write(
            root.path().join("tsconfig.json"),
            format!("{{\"compilerOptions\":{{\"baseUrl\":\".\"}},\"version\":{i}}}"),
        )?;
        sample(&mut values, "config_invalidation", i, || {
            index(&engine, root.path())
        });
    }
    let (sender, receiver) = mpsc::channel();
    let watcher = WatchHandle::start(root.path(), data.path().to_owned(), move |result| {
        let _ = sender.send(result);
    })?;
    receiver.recv_timeout(Duration::from_secs(10))??;
    for i in 0..WARMUPS + MEASURED {
        let marker = format!("watch_{i}");
        fs::write(
            root.path().join("module0.ts"),
            format!(
                "export function value() {{ return 1; }}\nexport function {marker}() {{ return 2; }}\n"
            ),
        )?;
        let started = Instant::now();
        loop {
            let snapshot = receiver.recv_timeout(Duration::from_secs(10))??;
            if snapshot.nodes.iter().any(|node| node.name == marker) {
                break;
            }
        }
        if i >= WARMUPS {
            values
                .entry("watcher_refresh".into())
                .or_default()
                .push(started.elapsed().as_secs_f64() * 1000.);
        }
    }
    drop(watcher);
    let database_bytes = fs::metadata(engine.status()?.database_path)?.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "fixture": "300 linked TypeScript modules, a re-export barrel, one consumer, ten tests, and tsconfig",
            "warmups": WARMUPS,
            "measured": MEASURED,
            "samples_ms": values,
        "peak_working_set_bytes": peak_working_set_bytes(),
            "database_bytes": database_bytes,
        }))?
    );
    Ok(())
}
