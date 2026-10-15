#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod export;

use std::sync::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};
use threadmark_core::{Engine, Status, Theme};
use threadmark_core::{
    graph::{Overview, Slice, TreeEntry},
    model::{Diagnostic, Node, Progress},
};

type AppState = Mutex<Result<Engine, String>>;
type WatchState = Mutex<Option<threadmark_core::watch::WatchHandle>>;
struct IndexJob {
    cancelled: Arc<AtomicBool>,
    running: AtomicBool,
}

fn with_engine<T>(
    state: &AppState,
    action: impl FnOnce(&Engine) -> Result<T, threadmark_core::Error>,
) -> Result<T, String> {
    let mut guard = state
        .lock()
        .map_err(|_| "Local storage lock failed; restart Threadmark".to_string())?;
    if guard.is_err() {
        *guard = Engine::open(None).map_err(|error| error.to_string());
    }
    let engine = guard.as_ref().map_err(Clone::clone)?;
    action(engine).map_err(|error| error.to_string())
}

async fn engine_task<T: Send + 'static>(
    app: tauri::AppHandle,
    action: impl FnOnce(&Engine) -> Result<T, threadmark_core::Error> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        with_engine(&state, action)
    })
    .await
    .map_err(|error| format!("Background analysis failed: {error}"))?
}

#[tauri::command]
async fn get_status(app: tauri::AppHandle) -> Result<Status, String> {
    engine_task(app, Engine::status).await
}

#[tauri::command]
async fn set_theme(theme: Theme, app: tauri::AppHandle) -> Result<Status, String> {
    engine_task(app, move |engine| engine.set_theme(theme)).await
}

#[tauri::command]
async fn choose_repository() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Open repository")
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn recent_repositories(app: tauri::AppHandle) -> Result<Vec<(String, i64)>, String> {
    engine_task(app, Engine::repositories).await
}

#[tauri::command]
async fn overview(app: tauri::AppHandle) -> Result<Overview, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| Ok(graph.overview()))
    })
    .await
}

#[tauri::command]
async fn select_repository(path: String, app: tauri::AppHandle) -> Result<Overview, String> {
    engine_task(app, move |engine| {
        engine.select_repository(std::path::Path::new(&path))
    })
    .await
}

#[tauri::command]
async fn index_repository(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    job: tauri::State<'_, IndexJob>,
) -> Result<Overview, String> {
    if job.running.swap(true, Ordering::SeqCst) {
        return Err("An index run is already active".into());
    }
    job.cancelled.store(false, Ordering::SeqCst);
    let cancelled = job.cancelled.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let engine = Engine::open(None).map_err(|error| error.to_string())?;
        engine
            .index_repository(
                std::path::Path::new(&path),
                &cancelled,
                |progress: &Progress| {
                    let _ = app.emit("index-progress", progress);
                },
            )
            .map_err(|error| error.to_string())?;
        engine
            .query(|graph| Ok(graph.overview()))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result);
    job.running.store(false, Ordering::SeqCst);
    if let Ok(overview) = &result {
        with_engine(&state, |engine| {
            engine.select_repository(std::path::Path::new(&overview.root))
        })?;
    }
    result
}

#[tauri::command]
fn cancel_index(job: tauri::State<'_, IndexJob>) {
    job.cancelled.store(true, Ordering::SeqCst);
}

#[tauri::command]
async fn search(query: String, app: tauri::AppHandle) -> Result<Vec<Node>, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| Ok(graph.search(&query, 100)))
    })
    .await
}

#[tauri::command]
async fn repository_tree(
    directory: String,
    app: tauri::AppHandle,
) -> Result<Vec<TreeEntry>, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| Ok(graph.tree(&directory)))
    })
    .await
}

#[tauri::command]
async fn graph_slice(
    selector: String,
    direction: String,
    depth: usize,
    kinds: Vec<String>,
    app: tauri::AppHandle,
) -> Result<Slice, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| graph.slice(&selector, &direction, depth, 24, &kinds))
    })
    .await
}

#[tauri::command]
async fn diagnostics(app: tauri::AppHandle) -> Result<Vec<Diagnostic>, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| {
            Ok(graph
                .snapshot
                .diagnostics
                .iter()
                .take(2000)
                .cloned()
                .collect())
        })
    })
    .await
}

#[tauri::command]
async fn find_path(from: String, to: String, app: tauri::AppHandle) -> Result<Slice, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| graph.find_path(&from, &to, 32))
    })
    .await
}

#[tauri::command]
async fn source_excerpt(
    selector: String,
    app: tauri::AppHandle,
) -> Result<threadmark_core::editor::Excerpt, String> {
    engine_task(app, move |engine| engine.excerpt(&selector)).await
}

#[tauri::command]
async fn open_in_editor(selector: String, app: tauri::AppHandle) -> Result<(), String> {
    engine_task(app, move |engine| engine.open_in_editor(&selector)).await
}

#[tauri::command]
async fn configure_editor(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let selected = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Select an installed editor executable")
            .pick_file()
    })
    .await
    .map_err(|error| error.to_string())?;
    if let Some(path) = selected {
        let executable = path.to_string_lossy().into_owned();
        with_engine(&state, |engine| {
            engine.configure_editor(threadmark_core::editor::Editor {
                executable: executable.clone(),
                arguments: vec!["--goto".into(), "{path}:{line}:{column}".into()],
            })
        })?;
        Ok(Some(executable))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn analyze_impact(
    selector: String,
    depth: usize,
    kinds: Vec<String>,
    app: tauri::AppHandle,
) -> Result<threadmark_core::impact::Impact, String> {
    engine_task(app, move |engine| {
        engine.query(|graph| graph.impact(&[selector], depth, &kinds))
    })
    .await
}

#[tauri::command]
async fn compare_changes(
    against: String,
    state: tauri::State<'_, AppState>,
    job: tauri::State<'_, IndexJob>,
) -> Result<threadmark_core::git::Comparison, String> {
    if job.running.swap(true, Ordering::AcqRel) {
        return Err("An indexing or comparison job is already running".into());
    }
    struct Reset<'a>(&'a AtomicBool);
    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _reset = Reset(&job.running);
    let root = with_engine(&state, |engine| Ok(engine.snapshot(None)?.root))?;
    let selected_root = root.clone();
    let comparison = tauri::async_runtime::spawn_blocking(move || {
        let engine = Engine::open(None).map_err(|error| error.to_string())?;
        engine
            .select_repository(std::path::Path::new(&root))
            .map_err(|error| error.to_string())?;
        engine.compare(&against).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    with_engine(&state, |engine| {
        engine.select_repository(std::path::Path::new(&selected_root))
    })?;
    Ok(comparison)
}

#[tauri::command]
async fn stop_watch(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WatchState>();

        let mut watcher = state.lock().map_err(|_| "Watcher lock failed")?;
        let previous = watcher.take();
        drop(previous);
        app.emit("watch-status", false)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Could not stop background watcher: {error}"))?
}
#[tauri::command]
async fn start_watch(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let watch = app.state::<WatchState>();

        let (root, data) = with_engine(&state, |engine| {
            Ok((engine.snapshot(None)?.root, engine.status()?.database_path))
        })?;
        let mut watcher = watch.lock().map_err(|_| "Watcher lock failed")?;
        let previous = watcher.take();
        drop(previous);
        let data = std::path::PathBuf::from(data)
            .parent()
            .ok_or("Invalid database directory")?
            .to_owned();
        let callback = app.clone();
        let watched_root = root.clone();
        let handle = threadmark_core::watch::WatchHandle::start(
            std::path::Path::new(&root),
            data,
            move |result| match result {
                Ok(_snapshot) => {
                    let state = callback.state::<AppState>();
                    let result = with_engine(&state, |engine| {
                        if engine.snapshot(None)?.root != watched_root {
                            return Err(threadmark_core::Error::Analysis(
                                "Repository changed while watching".into(),
                            ));
                        }
                        engine.select_repository(std::path::Path::new(&watched_root))
                    });
                    match result {
                        Ok(overview) => {
                            let _ = callback.emit("index-updated", overview);
                        }
                        Err(error) => {
                            let _ = callback.emit("watch-error", error);
                        }
                    }
                }
                Err(error) => {
                    let _ = callback.emit("watch-error", error.to_string());
                }
            },
        )
        .map_err(|error| error.to_string())?;
        *watcher = Some(handle);
        app.emit("watch-status", true)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Could not start background watcher: {error}"))?
}
#[tauri::command]
async fn export_metadata(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let snapshot = with_engine(&state, |engine| engine.export_graph(false))?;
        let json = serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?;
        let path = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("threadmark.json")
            .save_file();
        if let Some(path) = path {
            export::save(&path, &json).map_err(|error| format!("Could not save metadata export: {error}. Choose a writable folder with enough free space."))?;
            Ok(Some(path.to_string_lossy().into_owned()))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|error| error.to_string())?
}
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    // Keep the window available when storage fails so the user sees the actual
    // diagnostic instead of a disappearing application.
    let engine = Engine::open(None).map_err(|error| error.to_string());
    if let Err(error) = &engine {
        eprintln!("threadmark: {error}");
    }
    tauri::Builder::default()
        .manage(Mutex::new(engine))
        .manage(WatchState::new(None))
        .manage(IndexJob {
            cancelled: Arc::new(AtomicBool::new(false)),
            running: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_theme,
            choose_repository,
            recent_repositories,
            overview,
            select_repository,
            index_repository,
            cancel_index,
            search,
            repository_tree,
            graph_slice,
            diagnostics,
            find_path,
            source_excerpt,
            open_in_editor,
            configure_editor,
            analyze_impact,
            compare_changes,
            start_watch,
            stop_watch,
            export_metadata
        ])
        .run(tauri::generate_context!())
        .expect("Threadmark could not start its desktop runtime");
}
