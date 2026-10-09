use crate::{Engine, Error, indexing};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Editor {
    pub executable: String,
    pub arguments: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Excerpt {
    pub path: String,
    pub first_line: usize,
    pub lines: Vec<String>,
    pub stale: bool,
}

impl Engine {
    pub fn editor(&self) -> Result<Option<Editor>, Error> {
        self.database
            .setting("editor")?
            .map(|value| serde_json::from_str(&value).map_err(Error::from))
            .transpose()
    }
    pub fn configure_editor(&self, editor: Editor) -> Result<(), Error> {
        if !Path::new(&editor.executable).is_absolute() || !Path::new(&editor.executable).is_file()
        {
            return Err(Error::Analysis(
                "Editor must be the absolute path to an installed executable".into(),
            ));
        }
        if editor.arguments.len() > 32
            || editor
                .arguments
                .iter()
                .any(|argument| argument.len() > 4096)
        {
            return Err(Error::Analysis(
                "Editor arguments exceed their limit".into(),
            ));
        }
        self.database
            .set_setting("editor", &serde_json::to_string(&editor)?)?;
        Ok(())
    }
    pub fn excerpt(&self, selector: &str) -> Result<Excerpt, Error> {
        self.query(|graph| {
            let node = graph.select(selector)?;
            let source = indexing::safe_read(
                Path::new(&graph.snapshot.root),
                &node.path,
                indexing::MAX_FILE_BYTES,
            )
            .map_err(Error::Analysis)?;
            let first_line = node.span.line.saturating_sub(3).max(1);
            Ok(Excerpt {
                path: node.path.clone(),
                first_line,
                stale: graph
                    .snapshot
                    .nodes
                    .iter()
                    .find(|file| file.path == node.path && file.kind == "file")
                    .is_some_and(|file| file.fingerprint != crate::model::fingerprint(&source)),
                lines: source
                    .lines()
                    .skip(first_line - 1)
                    .take(9)
                    .map(|line| line.chars().take(500).collect())
                    .collect(),
            })
        })
    }
    pub fn open_in_editor(&self, selector: &str) -> Result<(), Error> {
        let editor = self.editor()?.ok_or_else(|| {
            Error::Analysis("Configure an installed editor executable in Settings first".into())
        })?;
        self.query(|graph| {
            let node = graph.select(selector)?;
            let root = Path::new(&graph.snapshot.root);
            let source = indexing::safe_read(root, &node.path, indexing::MAX_FILE_BYTES)
                .map_err(Error::Analysis)?;
            let path = root.join(&node.path).to_string_lossy().into_owned();
            let line = node.span.line.to_string();
            let source_line = source
                .lines()
                .nth(node.span.line.saturating_sub(1))
                .unwrap_or("");
            let column = source_line
                .get(..node.span.column.saturating_sub(1))
                .map(|prefix| prefix.encode_utf16().count() + 1)
                .unwrap_or(1)
                .to_string();
            let arguments: Vec<_> = editor
                .arguments
                .iter()
                .map(|argument| {
                    argument
                        .replace("{path}", &path)
                        .replace("{line}", &line)
                        .replace("{column}", &column)
                })
                .collect();
            std::process::Command::new(&editor.executable)
                .args(arguments)
                .current_dir(std::env::temp_dir())
                .spawn()
                .map_err(|error| {
                    Error::Analysis(format!(
                        "Could not launch editor {}: {error}",
                        editor.executable
                    ))
                })?;
            Ok(())
        })
    }
}
