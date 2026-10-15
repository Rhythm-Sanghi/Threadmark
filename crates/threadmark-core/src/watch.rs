use crate::{Engine, Error, model::Snapshot};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub struct WatchHandle {
    cancel: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn relevant(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let parts: Vec<_> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect();
    if parts.iter().any(|part| {
        matches!(
            part.as_ref(),
            "node_modules"
                | "dist"
                | "build"
                | "coverage"
                | "target"
                | "vendor"
                | ".venv"
                | "__pycache__"
        )
    }) {
        return false;
    }
    if parts.first().is_some_and(|part| part == ".git") {
        return parts.get(1).is_some_and(|part| {
            matches!(part.as_ref(), "HEAD" | "index" | "refs" | "packed-refs")
        });
    }
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    crate::parsing::language(&filename).is_some()
        || path.extension().is_none()
        || filename.ends_with(".json")
        || matches!(
            filename.as_ref(),
            ".gitignore" | ".threadmarkignore" | "pyproject.toml" | "pnpm-workspace.yaml"
        )
}
impl WatchHandle {
    pub fn start(
        root: &Path,
        data_directory: PathBuf,
        updated: impl FnMut(Result<Snapshot, Error>) + Send + 'static,
    ) -> Result<Self, Error> {
        Self::start_with_selection(root, data_directory, updated, true)
    }

    pub fn start_preserving_selection(
        root: &Path,
        data_directory: PathBuf,
        updated: impl FnMut(Result<Snapshot, Error>) + Send + 'static,
    ) -> Result<Self, Error> {
        Self::start_with_selection(root, data_directory, updated, false)
    }

    fn start_with_selection(
        root: &Path,
        data_directory: PathBuf,
        mut updated: impl FnMut(Result<Snapshot, Error>) + Send + 'static,
        select: bool,
    ) -> Result<Self, Error> {
        let root = crate::indexing::root(root)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let overflow = Arc::new(AtomicBool::new(false));
        let callback_overflow = overflow.clone();
        let (sender, receiver) = mpsc::sync_channel(64);
        let callback_root = root.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let accept = match &event {
                    Ok(event) => {
                        !matches!(event.kind, EventKind::Access(_))
                            && (event.need_rescan()
                                || event
                                    .paths
                                    .iter()
                                    .any(|path| relevant(path, &callback_root)))
                    }
                    Err(_) => true,
                };
                if accept && sender.try_send(event).is_err() {
                    callback_overflow.store(true, Ordering::Relaxed);
                }
            })
            .map_err(|error| {
                Error::Analysis(format!("Could not start filesystem watcher: {error}"))
            })?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|error| {
                Error::Analysis(format!("Could not watch {}: {error}", root.display()))
            })?;
        let worker = thread::spawn(move || {
            let _watcher = watcher;
            let engine = match Engine::open(Some(&data_directory)) {
                Ok(engine) => engine,
                Err(error) => {
                    updated(Err(error));
                    return;
                }
            };
            let mut pending = true;
            let mut since = Instant::now();
            let mut last = since;
            while !worker_cancel.load(Ordering::Relaxed) {
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(_)) => {
                        if !pending {
                            since = Instant::now();
                        }
                        pending = true;
                        last = Instant::now();
                    }
                    Ok(Err(error)) => {
                        updated(Err(Error::Analysis(format!("Filesystem watcher: {error}"))));
                        pending = true;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if overflow.swap(false, Ordering::Relaxed) {
                    updated(Err(Error::Analysis(
                        "Watcher event queue overflowed; refreshing the full index".into(),
                    )));
                    pending = true;
                }
                if pending
                    && (last.elapsed() >= Duration::from_millis(400)
                        || since.elapsed() >= Duration::from_secs(2))
                {
                    let result = if select {
                        engine.index_repository(&root, &worker_cancel, |_| {})
                    } else {
                        engine.index_repository_preserving_selection(&root, &worker_cancel, |_| {})
                    };
                    match result {
                        Err(Error::Cancelled) => break,
                        result => updated(result),
                    }
                    pending = false;
                }
            }
        });
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }
}
