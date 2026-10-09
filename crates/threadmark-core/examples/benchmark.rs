use std::{sync::atomic::AtomicBool, time::Instant};
use threadmark_core::Engine;
fn peak_memory() -> usize {
    #[cfg(windows)]
    {
        let mut counters =
            windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS::default();
        // Reads only this benchmark process, using the Windows memory DTO.
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runs = Vec::new();
    for functions in [1000, 10_000, 100_000] {
        let root = tempfile::tempdir()?;
        let data = tempfile::tempdir()?;
        let files = functions / 10;
        let source = (0..10)
            .map(|index| {
                if index == 9 {
                    format!("function f{index}(){{return 1;}}\n")
                } else {
                    format!(
                        "{}function f{index}(){{return f{}();}}\n",
                        if index == 0 { "export " } else { "" },
                        index + 1
                    )
                }
            })
            .collect::<String>();
        for index in 0..files {
            std::fs::write(root.path().join(format!("module{index}.ts")), &source)?;
        }
        let engine = Engine::open(Some(data.path()))?;
        let cancel = AtomicBool::new(false);
        let started = Instant::now();
        let snapshot = engine.index_repository(root.path(), &cancel, |_| {})?;
        let cold_ms = started.elapsed().as_secs_f64() * 1000.;
        let nodes = snapshot.nodes.len();
        let edges = snapshot.edges.len();
        drop(snapshot);
        let started = Instant::now();
        let snapshot = engine.index_repository(root.path(), &cancel, |_| {})?;
        let warm_ms = started.elapsed().as_secs_f64() * 1000.;
        let warm_reused = snapshot.progress.reused_files;
        drop(snapshot);
        std::fs::write(
            root.path().join("module0.ts"),
            source.replace("return 1", "return 2"),
        )?;
        let started = Instant::now();
        let snapshot = engine.index_repository(root.path(), &cancel, |_| {})?;
        let update_ms = started.elapsed().as_secs_f64() * 1000.;
        let parsed = snapshot.progress.parsed_files;
        let reused = snapshot.progress.reused_files;
        drop(snapshot);
        let started = Instant::now();
        engine.query(|_| Ok(()))?;
        let graph_load_ms = started.elapsed().as_secs_f64() * 1000.;
        let started = Instant::now();
        let search_count = engine.query(|graph| Ok(graph.search("f9", 100).len()))?;
        let search_ms = started.elapsed().as_secs_f64() * 1000.;
        let target = engine.query(|graph| {
            Ok(graph
                .snapshot
                .nodes
                .iter()
                .find(|node| node.path == "module0.ts" && node.name == "f9")
                .unwrap()
                .id
                .clone())
        })?;
        let started = Instant::now();
        let slice = engine.query(|graph| graph.slice(&target, "both", 2, 24, &[]))?;
        let slice_ms = started.elapsed().as_secs_f64() * 1000.;
        let started = Instant::now();
        let impact = engine.query(|graph| graph.impact(&[target], 8, &[]))?;
        let impact_ms = started.elapsed().as_secs_f64() * 1000.;
        let database_bytes = std::fs::metadata(engine.status()?.database_path)?.len();
        runs.push(serde_json::json!({"functions":functions,"files":files,"nodes":nodes,"edges":edges,"cold_ms":cold_ms,"unchanged_ms":warm_ms,"unchanged_reused":warm_reused,"one_file_update_ms":update_ms,"parsed":parsed,"reused":reused,"graph_load_ms":graph_load_ms,"search_ms":search_ms,"search_results":search_count,"peak_working_set_bytes":peak_memory(),"slice_ms":slice_ms,"slice_nodes":slice.nodes.len(),"impact_ms":impact_ms,"impact_results":impact.direct.len()+impact.transitive.len(),"impact_truncated":impact.truncated,"database_bytes":database_bytes}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema_version":1,"fixture":"10 functions per TS module; directed calls; one exported entry per module","runs":runs})
        )?
    );
    Ok(())
}
