use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};
use threadmark_core::{Engine, Theme, editor::Editor};

#[derive(Parser)]
#[command(
    name = "threadmark",
    version,
    about = "Local codebase relationship explorer and change-impact analyzer"
)]
struct Arguments {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    repository: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, conflicts_with = "json")]
    quiet: bool,
    #[arg(long, global = true)]
    no_color: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Index local source without executing repository code.
    Index {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Local storage and selected repository state.
    Status,
    /// Inspect mutually reachable local imports and their evidence.
    Cycles,
    /// Watch source changes with selective reparsing. Ctrl+C stops.
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Detect related tests through static relationships; never runs tests.
    Tests { target: String },
    /// Compare refreshed working-tree analysis against a local Git reference.
    Changes {
        #[arg(long, default_value = "HEAD")]
        against: String,
    },
    /// Search indexed files and symbols (maximum 100 results).
    Search { query: String },
    /// Inspect a node and relationships with evidence.
    Show { target: String },
    /// Show a bounded dependency neighborhood.
    Deps {
        target: String,
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    /// Show a bounded dependent neighborhood.
    Dependents {
        target: String,
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    /// Follow reverse relationships to detect change impact.
    Impact {
        #[arg(required_unless_present = "against")]
        target: Option<String>,
        #[arg(long, conflicts_with = "target")]
        against: Option<String>,
        #[arg(long, default_value_t = 8)]
        depth: usize,
        #[arg(long)]
        relationship: Vec<String>,
    },
    /// Find a directed relationship path between two selectors.
    Path { from: String, to: String },
    /// Show index blind spots.
    Diagnostics,
    /// Open source in the explicitly configured editor.
    Open { target: String },
    /// Export graph metadata; source evidence is omitted unless requested.
    Export {
        #[arg(long)]
        include_evidence: bool,
    },
    /// Configure appearance and an installed editor executable.
    Settings {
        #[arg(long)]
        theme: Option<Theme>,
        #[arg(long)]
        editor: Option<String>,
        #[arg(long, allow_hyphen_values = true)]
        editor_arg: Vec<String>,
    },
}

fn main() -> ExitCode {
    let args = Arguments::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("threadmark: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print<T: serde::Serialize>(
    args: &Arguments,
    value: &T,
    text: impl FnOnce() -> String,
) -> Result<(), serde_json::Error> {
    if args.json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else if !args.quiet {
        println!(
            "{}",
            text()
                .chars()
                .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
                .collect::<String>()
        );
    }
    Ok(())
}
fn nodes_text(nodes: &[threadmark_core::model::Node]) -> String {
    nodes
        .iter()
        .map(|node| {
            format!(
                "{}  {}  {}:{}\n  {}",
                node.name, node.kind, node.path, node.span.line, node.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn slice_text(slice: &threadmark_core::graph::Slice) -> String {
    let mut text = nodes_text(&slice.nodes);
    for edge in &slice.edges {
        text.push_str(&format!(
            "\n{} · {} · {}:{}\n  {}",
            edge.kind, edge.confidence, edge.path, edge.span.line, edge.evidence
        ));
    }
    if slice.truncated {
        text.push_str("\nResults truncated at the traversal limit.");
    }
    text
}

fn run(args: &Arguments) -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::open(args.data_dir.as_deref())?;
    if let Command::Watch { path } = &args.command {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = cancel.clone();
        ctrlc::set_handler(move || signal.store(true, std::sync::atomic::Ordering::Relaxed))?;
        let data = PathBuf::from(engine.status()?.database_path)
            .parent()
            .ok_or("Invalid database directory")?
            .to_owned();
        let (sender, receiver) = std::sync::mpsc::channel();
        let _watch = threadmark_core::watch::WatchHandle::start(path, data, move |result| {
            let _ = sender.send(result);
        })?;
        while !cancel.load(std::sync::atomic::Ordering::Relaxed) {
            match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(result) => {
                    let snapshot = result?;
                    let overview = threadmark_core::graph::Graph::new(snapshot).overview();
                    if args.json {
                        println!("{}", serde_json::to_string(&overview)?);
                    } else if !args.quiet {
                        println!(
                            "Updated {} · {} parsed · {} reused · {} relationships",
                            overview.root,
                            overview.progress.parsed_files,
                            overview.progress.reused_files,
                            overview.relationships
                        );
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(error) => return Err(error.into()),
            }
        }
        return Ok(());
    }
    if let Command::Index { path } = &args.command {
        engine.index_repository(path, &std::sync::atomic::AtomicBool::new(false), |_| {})?;
        let overview = engine.query(|graph| Ok(graph.overview()))?;
        print(args, &overview, || {
            format!(
                "Indexed {}\n{} source files · {} symbols · {} relationships · {} diagnostics",
                overview.root,
                overview.source_files,
                overview.symbols,
                overview.relationships,
                overview.diagnostics
            )
        })?;
        return Ok(());
    }
    if let Some(path) = &args.repository {
        engine.select_repository(path)?;
    }
    match &args.command {
        Command::Cycles => {
            let cycles = engine.query(|graph| Ok(graph.cycles()))?;
            print(
                args,
                &serde_json::json!({"schema_version":1,"cycles":cycles,"limit":100}),
                || {
                    cycles
                        .iter()
                        .map(slice_text)
                        .collect::<Vec<_>>()
                        .join("\n\n")
                },
            )?;
        }
        Command::Tests { target } => {
            let impact =
                engine.query(|graph| graph.impact(std::slice::from_ref(target), 8, &[]))?;
            print(
                args,
                &serde_json::json!({"schema_version":1,"detected_related_tests":impact.related_tests,"truncated":impact.truncated}),
                || {
                    nodes_text(
                        &impact
                            .related_tests
                            .iter()
                            .map(|item| item.node.clone())
                            .collect::<Vec<_>>(),
                    )
                },
            )?;
        }
        Command::Changes { against } => {
            let comparison = engine.compare(against)?;
            print(args, &comparison, || {
                format!(
                    "Against {} ({})\n{} changed source files\n{} symbol changes\n{} added relationships · {} removed relationships",
                    comparison.baseline,
                    comparison.baseline_commit,
                    comparison.changed_files.len(),
                    comparison.symbol_change_count,
                    comparison.added_relationship_count,
                    comparison.removed_relationship_count
                )
            })?;
        }
        Command::Status => {
            let status = engine.status()?;
            let repository = engine.selected_overview()?;
            let output = serde_json::json!({"schema_version":1,"app_version":status.app_version,"database_schema":status.database_schema,"database_path":status.database_path,"theme":status.theme,"analysis_available":status.analysis_available,"repository":repository});
            print(args, &output, || {
                format!(
                    "Threadmark {}\nDatabase: {}\nSchema: {}\nTheme: {}\n{}",
                    status.app_version,
                    status.database_path,
                    status.database_schema,
                    status.theme.as_str(),
                    repository
                        .map(|r| format!(
                            "{} · {} files · {} symbols · {} relationships",
                            r.root, r.source_files, r.symbols, r.relationships
                        ))
                        .unwrap_or("No selected index. Run threadmark index <path>.".into())
                )
            })?;
        }
        Command::Settings {
            theme,
            editor,
            editor_arg,
        } => {
            if let Some(theme) = theme {
                engine.set_theme(*theme)?;
            }
            if let Some(executable) = editor {
                engine.configure_editor(Editor {
                    executable: executable.clone(),
                    arguments: if editor_arg.is_empty() {
                        vec!["--goto".into(), "{path}:{line}:{column}".into()]
                    } else {
                        editor_arg.clone()
                    },
                })?;
            }
            let status = engine.status()?;
            let editor = engine.editor()?;
            print(args, &status, || {
                format!("Theme: {}\nEditor: {:?}", status.theme.as_str(), editor)
            })?;
        }
        Command::Search { query } => {
            let nodes = engine.query(|graph| Ok(graph.search(query, 100)))?;
            print(
                args,
                &serde_json::json!({"schema_version":1,"results":nodes}),
                || nodes_text(&nodes),
            )?;
        }
        Command::Show { target }
        | Command::Deps { target, depth: _ }
        | Command::Dependents { target, depth: _ } => {
            let (direction, depth) = match &args.command {
                Command::Deps { depth, .. } => ("dependencies", *depth),
                Command::Dependents { depth, .. } => ("dependents", *depth),
                _ => ("both", 1),
            };
            let slice = engine.query(|graph| graph.slice(target, direction, depth, 120, &[]))?;
            print(args, &slice, || slice_text(&slice))?;
        }
        Command::Impact {
            target,
            against,
            depth,
            relationship,
        } => {
            if let Some(reference) = against {
                let comparison = engine.compare(reference)?;
                print(args, &comparison, || {
                    format!(
                        "Against {}\n{} changed source files · {} symbol changes\n{} current direct dependents · {} baseline direct dependents of removed symbols",
                        reference,
                        comparison.changed_files.len(),
                        comparison.symbol_change_count,
                        comparison.current_impact.direct.len(),
                        comparison.removed_impact.direct.len()
                    )
                })?;
                return Ok(());
            }
            let target = target
                .as_ref()
                .ok_or("Pass a target or --against <reference>")?;
            let impact = engine
                .query(|graph| graph.impact(std::slice::from_ref(target), *depth, relationship))?;
            print(args, &impact, || {
                let mut text = String::new();
                for (name, items) in [
                    ("Direct", &impact.direct),
                    ("Transitive", &impact.transitive),
                    ("Possible", &impact.possible),
                    ("Detected related tests", &impact.related_tests),
                ] {
                    text.push_str(&format!("{name}\n"));
                    for item in items {
                        text.push_str(&format!(
                            "  {} · {}:{} · {} hops\n",
                            item.node.qualified_name,
                            item.node.path,
                            item.node.span.line,
                            item.hops
                        ));
                    }
                }
                if impact.truncated {
                    text.push_str("Results truncated; increase depth or narrow the target.\n");
                }
                text
            })?;
        }
        Command::Path { from, to } => {
            let slice = engine.query(|graph| graph.find_path(from, to, 32))?;
            print(args, &slice, || {
                if slice.nodes.is_empty() {
                    "No detected directed path within the traversal limits.".into()
                } else {
                    slice_text(&slice)
                }
            })?;
        }
        Command::Diagnostics => {
            let diagnostics = engine.query(|graph| Ok(graph.snapshot.diagnostics.clone()))?;
            print(
                args,
                &serde_json::json!({"schema_version":1,"diagnostics":diagnostics}),
                || {
                    diagnostics
                        .iter()
                        .map(|item| {
                            format!(
                                "{}:{} [{}] {}",
                                item.path, item.line, item.code, item.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                },
            )?;
        }
        Command::Open { target } => engine.open_in_editor(target)?,
        Command::Export { include_evidence } => {
            let snapshot = engine.export_graph(*include_evidence)?;
            if !args.quiet {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            }
        }
        Command::Index { .. } => unreachable!(),
        Command::Watch { .. } => unreachable!(),
    }
    Ok(())
}
