use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use threadmark_db::Database;
pub mod cycles;
pub mod editor;
pub mod git;
pub mod graph;
pub mod impact;
pub mod indexing;
pub mod model;
pub mod parsing;
pub mod python;
pub mod resolution;
pub mod watch;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Analysis(String),
    #[error("Indexing cancelled; the previous index was preserved")]
    Cancelled,
    #[error("Index data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Could not determine the local data directory; pass --data-dir")]
    DataDirectory,
    #[error("Could not create data directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Database(#[from] threadmark_db::Error),
    #[error("Invalid saved theme {0:?}; expected system, light or dark")]
    InvalidTheme(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl std::str::FromStr for Theme {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Error> {
        match value {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(Error::InvalidTheme(value.into())),
        }
    }
}

// This is the versioned client contract, separate from database representation.
#[derive(Debug, Serialize)]
pub struct Status {
    pub schema_version: u32,
    pub app_version: &'static str,
    pub database_schema: u32,
    pub database_path: String,
    pub theme: Theme,
    pub analysis_available: bool,
}

pub struct Engine {
    database: Database,
    database_path: PathBuf,
    graph: std::cell::RefCell<Option<graph::Graph>>,
    active_root: std::cell::RefCell<Option<String>>,
    graph_revision: std::cell::Cell<i64>,
}

impl Engine {
    pub fn open(data_dir: Option<&Path>) -> Result<Self, Error> {
        let directory = match data_dir {
            Some(path) => path.to_owned(),
            None => directories::ProjectDirs::from("org", "Threadmark", "Threadmark")
                .ok_or(Error::DataDirectory)?
                .data_local_dir()
                .to_owned(),
        };
        std::fs::create_dir_all(&directory).map_err(|source| Error::CreateDirectory {
            path: directory.clone(),
            source,
        })?;
        let database_path = directory.join("threadmark.db");
        let database = Database::open(&database_path)?;
        tracing::info!(
            schema = threadmark_db::SCHEMA_VERSION,
            "Local storage ready"
        );
        Ok(Self {
            database,
            database_path,
            graph: std::cell::RefCell::new(None),
            active_root: std::cell::RefCell::new(None),
            graph_revision: std::cell::Cell::new(0),
        })
    }

    pub fn status(&self) -> Result<Status, Error> {
        let theme = self
            .database
            .setting("theme")?
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or_default();
        Ok(Status {
            schema_version: 1,
            app_version: env!("CARGO_PKG_VERSION"),
            database_schema: self.database.schema_version()?,
            database_path: self.database_path.to_string_lossy().into_owned(),
            theme,
            analysis_available: true,
        })
    }

    pub fn set_theme(&self, theme: Theme) -> Result<Status, Error> {
        self.database.set_setting("theme", theme.as_str())?;
        self.status()
    }

    pub fn index_repository(
        &self,
        path: &Path,
        cancel: &std::sync::atomic::AtomicBool,
        mut progress: impl FnMut(&model::Progress),
    ) -> Result<model::Snapshot, Error> {
        let root = indexing::root(path)?;
        // The persisted index remains available on cancellation. Release the old
        // query graph before constructing a replacement to limit peak memory.
        *self.graph.borrow_mut() = None;
        let root_name = indexing::root_string(&root);
        let mut cache: std::collections::HashMap<String, (String, model::ParsedFile)> = self
            .database
            .cached_files(&root_name, parsing::ADAPTER_VERSION)?
            .into_iter()
            .filter_map(|(path, hash, parsed)| {
                serde_json::from_str(&parsed)
                    .ok()
                    .map(|parsed| (path, (hash, parsed)))
            })
            .collect();
        let cached_hashes: std::collections::HashMap<_, _> = cache
            .iter()
            .map(|(path, (hash, _))| (path.clone(), hash.clone()))
            .collect();
        let (mut snapshot, files) =
            indexing::discover_and_parse(&root, &mut cache, cancel, &mut progress)?;
        drop(cache);
        resolution::resolve(&root, &files, &mut snapshot);
        snapshot.diagnostics.extend(cycles::diagnostics(&snapshot));
        snapshot.progress.relationships = snapshot.edges.len();
        progress(&snapshot.progress);
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let stored = files
            .iter()
            .map(|file| {
                Ok((
                    file.path.clone(),
                    file.hash.clone(),
                    if cached_hashes.get(&file.path) == Some(&file.hash) {
                        None
                    } else {
                        Some(serde_json::to_string(file)?)
                    },
                ))
            })
            .collect::<Result<Vec<_>, serde_json::Error>>()?;
        let snapshot_json = serde_json::to_string(&snapshot)?;
        // Serialization can take seconds for a large graph. Honour a cancel
        // request received during that work before entering the transaction.
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        self.database.save_index(
            &root_name,
            snapshot.indexed_at as i64,
            &snapshot_json,
            parsing::ADAPTER_VERSION,
            &stored,
        )?;
        // Query adjacency is loaded lazily; do not duplicate the returned graph.
        *self.graph.borrow_mut() = None;
        *self.active_root.borrow_mut() = Some(root_name);
        Ok(snapshot)
    }

    pub fn snapshot(&self, path: Option<&Path>) -> Result<model::Snapshot, Error> {
        let root = match path {
            Some(path) => indexing::root_string(&indexing::root(path)?),
            None => self
                .active_root
                .borrow()
                .clone()
                .or(self.database.setting("repository")?)
                .ok_or_else(|| {
                    Error::Analysis("No repository indexed. Run threadmark index <path>".into())
                })?,
        };
        let json = self.database.snapshot(&root)?.ok_or_else(|| {
            Error::Analysis(format!("No index for {root}. Run threadmark index <path>"))
        })?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn repositories(&self) -> Result<Vec<(String, i64)>, Error> {
        Ok(self.database.repositories()?)
    }
    pub fn export_graph(&self, include_evidence: bool) -> Result<model::Snapshot, Error> {
        let mut snapshot = self.snapshot(None)?;
        if !include_evidence {
            for edge in &mut snapshot.edges {
                edge.evidence.clear();
            }
            for diagnostic in &mut snapshot.diagnostics {
                diagnostic.message.clear();
            }
        }
        Ok(snapshot)
    }
    pub fn selected_overview(&self) -> Result<Option<graph::Overview>, Error> {
        if self.active_root.borrow().is_none() && self.database.setting("repository")?.is_none() {
            return Ok(None);
        }
        self.query(|graph| Ok(Some(graph.overview())))
    }

    pub fn query<T>(
        &self,
        action: impl FnOnce(&graph::Graph) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let root = self
            .active_root
            .borrow()
            .clone()
            .or(self.database.setting("repository")?)
            .ok_or_else(|| {
                Error::Analysis("No repository indexed. Run threadmark index <path>".into())
            })?;
        let revision = self.database.index_revision(&root)?.unwrap_or(0);
        *self.active_root.borrow_mut() = Some(root);
        if self.graph.borrow().is_none() || revision != self.graph_revision.get() {
            let snapshot = self.snapshot(None)?;
            *self.graph.borrow_mut() = Some(graph::Graph::new(snapshot));
            self.graph_revision.set(revision);
        }
        let borrowed = self.graph.borrow();
        action(borrowed.as_ref().unwrap())
    }

    pub fn select_repository(&self, path: &Path) -> Result<graph::Overview, Error> {
        let snapshot = self.snapshot(Some(path))?;
        self.database.set_setting("repository", &snapshot.root)?;
        self.graph_revision
            .set(self.database.index_revision(&snapshot.root)?.unwrap_or(0));
        *self.active_root.borrow_mut() = Some(snapshot.root.clone());
        *self.graph.borrow_mut() = Some(graph::Graph::new(snapshot));
        self.query(|graph| Ok(graph.overview()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_engine_persists_theme_and_reports_real_storage() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(Some(directory.path())).unwrap();
        assert_eq!(engine.status().unwrap().theme, Theme::System);
        engine.set_theme(Theme::Dark).unwrap();
        drop(engine);
        let status = Engine::open(Some(directory.path()))
            .unwrap()
            .status()
            .unwrap();
        assert_eq!(status.theme, Theme::Dark);
        assert!(Path::new(&status.database_path).is_file());
        assert!(status.analysis_available);
    }
    #[test]
    fn invalid_theme_is_rejected() {
        assert!("purple".parse::<Theme>().is_err());
    }
    #[test]
    fn data_directory_errors_are_reported() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(matches!(
            Engine::open(Some(file.path())),
            Err(Error::CreateDirectory { .. })
        ));
    }
}
