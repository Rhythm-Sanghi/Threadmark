use std::{sync::atomic::AtomicBool, time::Instant};
use threadmark_core::Engine;

fn memory() -> usize {
    #[cfg(windows)]
    {
        let mut counters =
            windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS::default();
        let success = unsafe {
            windows_sys::Win32::System::ProcessStatus::GetProcessMemoryInfo(
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                &mut counters,
                std::mem::size_of_val(&counters) as u32,
            )
        };
        if success == 0 {
            0
        } else {
            counters.WorkingSetSize
        }
    }
    #[cfg(not(windows))]
    {
        0
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let data = tempfile::tempdir()?;
    let source = (0..10)
        .map(|index| {
            format!(
                "{}function f{index}(){{return {};}}\n",
                if index == 0 { "export " } else { "" },
                if index < 9 {
                    format!("f{}()", index + 1)
                } else {
                    "1".into()
                }
            )
        })
        .collect::<String>();
    for index in 0..1000 {
        std::fs::write(root.path().join(format!("module{index}.ts")), &source)?;
    }
    let engine = Engine::open(Some(data.path()))?;
    let cancel = AtomicBool::new(false);
    let start = Instant::now();
    let mut samples = Vec::new();
    for iteration in 0..120 {
        std::fs::write(
            root.path().join("module0.ts"),
            format!("{source}\n// iteration {iteration}"),
        )?;
        let snapshot = engine.index_repository(root.path(), &cancel, |_| {})?;
        if snapshot.nodes.len() != 11000 || (iteration > 0 && snapshot.progress.reused_files != 999)
        {
            return Err("Unexpected index/cache result during soak".into());
        }
        drop(snapshot);
        engine.query(|graph| {
            let target = graph.select("module0.ts#f9")?;
            let impact = graph.impact(std::slice::from_ref(&target.id), 8, &[])?;
            if impact.direct.len() + impact.transitive.len() != 8 {
                return Err(threadmark_core::Error::Analysis(
                    "Impact changed during soak".into(),
                ));
            }
            Ok(())
        })?;
        if iteration % 10 == 9 {
            samples.push(serde_json::json!({"iteration":iteration+1,"working_set_bytes":memory()}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema_version":1,"cycles":120,"files":1000,"functions":10000,"elapsed_seconds":start.elapsed().as_secs_f64(),"samples":samples,"scope":"Repeated index/cache/adjacency/impact lifecycle. Not a multi-hour native UI soak."})
        )?
    );
    Ok(())
}
