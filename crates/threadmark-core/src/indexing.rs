use crate::{
    Error,
    model::*,
    parsing::{self, JavaScript, LanguageAdapter},
};
use ignore::WalkBuilder;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_FILES: usize = 50_000;

pub fn root(path: &Path) -> Result<PathBuf, Error> {
    let root = path
        .canonicalize()
        .map_err(|error| Error::Analysis(format!("Could not open {}: {error}", path.display())))?;
    if !root.is_dir() {
        return Err(Error::Analysis(format!(
            "{} is not a directory",
            path.display()
        )));
    }
    Ok(root)
}

pub fn root_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn safe_read(root: &Path, relative: &str, max: u64) -> Result<String, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("Path must be relative to the repository".into());
    }
    let mut current = root.to_owned();
    for part in relative_path.components() {
        current.push(part);
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| error.to_string())?;
        if is_link(&metadata) {
            return Err("Symlink or reparse-point paths are not followed".into());
        }
    }
    let canonical = current.canonicalize().map_err(|error| error.to_string())?;
    if !canonical.starts_with(root) {
        return Err("Path escapes the repository".into());
    }
    let file = std::fs::File::open(&canonical).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        let mut buffer = vec![0u16; 32768];
        // Verify the opened handle, not another path lookup, before reading bytes.
        // This closes the junction-swap gap between canonicalization and opening.
        let count = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(
                file.as_raw_handle() as _,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        } as usize;
        if count == 0 || count >= buffer.len() {
            return Err("Could not verify opened source handle boundary".into());
        }
        let opened = PathBuf::from(String::from_utf16_lossy(&buffer[..count]));
        if !opened.starts_with(root) {
            return Err("Opened source handle escapes the repository".into());
        }
    }
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("Not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > max {
        return Err(format!("File exceeds the {} byte limit", max));
    }
    String::from_utf8(bytes).map_err(|_| "File is not valid UTF-8".into())
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn excluded(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "dist"
            | "build"
            | "coverage"
            | ".next"
            | "vendor"
            | "target"
            | ".venv"
            | "__pycache__"
    )
}

// Keep traversal decisions in the visitor: Skip prunes a boundary and Quit
// stops enumeration even when every remaining entry would have been rejected.
// A single walker thread preserves bounded memory and predictable disk pressure.
type DiscoveryEntries = Vec<Result<ignore::DirEntry, ignore::Error>>;

fn discover(
    root: &Path,
    cancel: &AtomicBool,
    max_entries: usize,
    max_depth: usize,
) -> Result<(DiscoveryEntries, Vec<Diagnostic>), Error> {
    use ignore::WalkState;
    let collected = std::sync::Mutex::new((Vec::new(), Vec::new(), 0usize));
    WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .max_depth(Some(max_depth))
        .threads(1)
        .add_custom_ignore_filename(".threadmarkignore")
        .build_parallel()
        .run(|| Box::new(|entry| {
            if cancel.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let mut state = collected.lock().expect("discovery state");
            let (entries, diagnostics, visited) = &mut *state;
            if entry.as_ref().is_ok_and(|entry| entry.depth() == 0) {
                return WalkState::Continue;
            }
            if *visited == max_entries {
                diagnostics.push(Diagnostic {
                    path: String::new(), line: 0, code: "entry-limit".into(),
                    message: format!("Repository discovery exceeds the {max_entries} entry limit; remaining entries were skipped"),
                });
                return WalkState::Quit;
            }
            *visited += 1;
            if let Ok(entry) = &entry {
                if excluded(&entry.file_name().to_string_lossy()) {
                    return WalkState::Skip;
                }
                let code = match entry.path().symlink_metadata() {
                    Ok(metadata) if is_link(&metadata) => Some("link-skipped"),
                    Ok(metadata) if metadata.is_dir() && entry.depth() >= max_depth => Some("depth-limit"),
                    Err(_) => Some("metadata-error"),
                    _ => None,
                };
                if let Some(code) = code {
                    diagnostics.push(Diagnostic {
                        path: entry.path().strip_prefix(root).unwrap_or(entry.path()).to_string_lossy().replace('\\', "/"),
                        line: 0, code: code.into(),
                        message: "Entry excluded by repository boundary or discovery limit".into(),
                    });
                    return WalkState::Skip;
                }
                if !entry.file_type().is_some_and(|kind| kind.is_file()) && entry.error().is_none() {
                    return WalkState::Continue;
                }
            }
            entries.push(entry);
            WalkState::Continue
        }));
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    let (entries, diagnostics, _) = collected.into_inner().expect("discovery state");
    Ok((entries, diagnostics))
}

pub fn discover_and_parse(
    root: &Path,
    cache: &mut HashMap<String, (String, ParsedFile)>,
    cancel: &AtomicBool,
    progress: &mut impl FnMut(&Progress),
) -> Result<(Snapshot, Vec<ParsedFile>), Error> {
    let root_name = root_string(root);
    let mut state = Progress {
        phase: "Discovering files".into(),
        ..Progress::default()
    };
    let mut diagnostics = Vec::new();
    let mut files = Vec::new();
    let mut project_files = Vec::new();
    let mut languages = std::collections::BTreeSet::new();
    let mut total = 0;
    let mut references = 0usize;
    let (entries, skipped) = discover(root, cancel, 200_000, 64)?;
    diagnostics.extend(skipped);
    if root.join(".git").exists() {
        project_files.push(".git".into());
    }
    for entry in entries {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    path: String::new(),
                    line: 0,
                    code: "discovery-error".into(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        if let Some(error) = entry.error() {
            diagnostics.push(Diagnostic {
                path: entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/"),
                line: 0,
                code: "ignore-error".into(),
                message: error.to_string(),
            });
        }
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        state.examined += 1;
        if state.examined > MAX_FILES {
            diagnostics.push(Diagnostic {
                path: String::new(),
                line: 0,
                code: "file-limit".into(),
                message: "Repository exceeds the 50,000 file limit; remaining files were skipped"
                    .into(),
            });
            break;
        }
        let path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        let basename = entry.file_name().to_string_lossy();
        if matches!(
            basename.as_ref(),
            "package.json"
                | "tsconfig.json"
                | "jsconfig.json"
                | "pnpm-workspace.yaml"
                | "yarn.lock"
                | "package-lock.json"
                | "pyproject.toml"
        ) || basename.starts_with("vitest.config.")
            || basename.starts_with("jest.config.")
        {
            project_files.push(path.clone());
        }
        let Some(language) = parsing::language(&path) else {
            if matches!(
                path.rsplit('.').next(),
                Some("py" | "rs" | "go" | "java" | "cs" | "cpp" | "rb" | "php")
            ) {
                diagnostics.push(Diagnostic {
                    path,
                    line: 0,
                    code: "unsupported-language".into(),
                    message: "Detected, but detailed analysis is not supported yet".into(),
                });
            }
            continue;
        };
        state.source_files += 1;
        state.current_path = path.clone();
        state.phase = format!("Parsing {language}");
        languages.insert(language.to_owned());
        let source = match safe_read(root, &path, MAX_FILE_BYTES) {
            Ok(source) => source,
            Err(message) => {
                diagnostics.push(Diagnostic {
                    path,
                    line: 0,
                    code: "read-error".into(),
                    message,
                });
                continue;
            }
        };
        total += source.len() as u64;
        if total > MAX_TOTAL_BYTES {
            diagnostics.push(Diagnostic {
                path,
                line: 0,
                code: "size-limit".into(),
                message:
                    "Repository exceeds the 512 MiB source limit; remaining files were skipped"
                        .into(),
            });
            break;
        }
        let hash = blake3::hash(source.as_bytes()).to_hex().to_string();
        let parsed = if let Some((cached_hash, parsed)) = cache
            .remove(&path)
            .filter(|(cached_hash, _)| *cached_hash == hash)
        {
            let _ = cached_hash;
            state.reused_files += 1;
            parsed
        } else {
            state.parsed_files += 1;
            match JavaScript.parse(&root_name, &path, &source) {
                Ok(parsed) => parsed,
                Err(message) => {
                    diagnostics.push(Diagnostic {
                        path,
                        line: 0,
                        code: "parser-error".into(),
                        message,
                    });
                    continue;
                }
            }
        };
        if state.symbols + files.len() + parsed.nodes.len() > 200_000
            || references + parsed.references.len() + parsed.imports.len() > 500_000
        {
            diagnostics.push(Diagnostic{path:parsed.path.clone(),line:0,code:"graph-limit".into(),message:"Index exceeds 200,000 nodes or 500,000 syntax references; remaining files were omitted".into()});
            break;
        }
        references += parsed.references.len() + parsed.imports.len();
        state.symbols += parsed.nodes.len().saturating_sub(1);
        diagnostics.extend(parsed.diagnostics.iter().cloned());
        files.push(parsed);
        progress(&state);
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let nodes = files
        .iter()
        .flat_map(|file| file.nodes.iter().cloned())
        .collect();
    state.phase = "Saving index".into();
    progress(&state);
    let snapshot = Snapshot {
        schema_version: 1,
        root: root_name,
        name: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        indexed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        nodes,
        edges: Vec::new(),
        diagnostics,
        project_files,
        languages: languages.into_iter().collect(),
        progress: state,
    };
    Ok((snapshot, files))
}

#[cfg(test)]
mod discovery_tests {
    use super::*;

    #[test]
    fn exact_limit_is_complete_and_one_more_is_explicitly_partial() {
        let root = tempfile::tempdir().unwrap();
        for name in ["a.ts", "b.ts"] {
            std::fs::write(root.path().join(name), "function work() {}").unwrap();
        }
        let cancel = AtomicBool::new(false);
        let (entries, diagnostics) = discover(root.path(), &cancel, 2, 64).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(diagnostics.is_empty());
        std::fs::write(root.path().join("c.ts"), "function other() {}").unwrap();
        let (entries, diagnostics) = discover(root.path(), &cancel, 2, 64).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.code == "entry-limit")
                .count(),
            1
        );
    }

    #[test]
    fn rejected_entries_count_without_traversing_their_children() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..20 {
            let folder = root.path().join(format!("p{index}/node_modules"));
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("secret.ts"), "function secret() {}").unwrap();
        }
        let (entries, diagnostics) = discover(root.path(), &AtomicBool::new(false), 8, 64).unwrap();
        assert!(entries.is_empty());
        assert!(diagnostics.iter().any(|d| d.code == "entry-limit"));
        let (entries, diagnostics) =
            discover(root.path(), &AtomicBool::new(false), 40, 64).unwrap();
        assert!(entries.is_empty());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn depth_and_pre_cancel_are_enforced_even_without_source_files() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("a/b/c/d")).unwrap();
        std::fs::write(root.path().join("a/b/c/d/work.ts"), "function work() {}").unwrap();
        let (entries, diagnostics) =
            discover(root.path(), &AtomicBool::new(false), 100, 3).unwrap();
        assert!(entries.is_empty());
        assert!(diagnostics.iter().any(|d| d.code == "depth-limit"));
        assert!(matches!(
            discover(root.path(), &AtomicBool::new(true), 100, 64),
            Err(Error::Cancelled)
        ));
    }
}
