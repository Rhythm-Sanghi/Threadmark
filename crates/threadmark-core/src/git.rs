use crate::{
    Engine, Error,
    graph::Graph,
    impact::Impact,
    model::*,
    parsing::{self, JavaScript, LanguageAdapter},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    sync::atomic::AtomicBool,
};

#[derive(Debug, Serialize)]
pub struct SymbolChange {
    pub kind: String,
    pub node: Node,
}
#[derive(Debug, Serialize)]
pub struct Comparison {
    pub schema_version: u32,
    pub baseline: String,
    pub baseline_commit: String,
    pub branch: String,
    pub changed_files: Vec<String>,
    pub symbol_changes: Vec<SymbolChange>,
    pub added_relationships: Vec<Edge>,
    pub removed_relationships: Vec<Edge>,
    pub current_impact: Impact,
    pub removed_impact: Impact,
    pub diagnostics: Vec<Diagnostic>,
    pub symbol_change_count: usize,
    pub added_relationship_count: usize,
    pub removed_relationship_count: usize,
    pub truncated: bool,
}
fn failure(error: impl std::fmt::Display) -> Error {
    Error::Analysis(format!("Git comparison: {error}"))
}

fn baseline(root: &Path, reference: &str) -> Result<(String, String, Snapshot), Error> {
    if reference.is_empty() || reference.len() > 256 {
        return Err(failure("Invalid baseline reference"));
    }
    let repo = git2::Repository::discover(root).map_err(failure)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| failure("Bare repositories have no working tree"))?
        .canonicalize()
        .map_err(failure)?;
    let prefix = root
        .strip_prefix(&workdir)
        .map_err(failure)?
        .to_string_lossy()
        .replace('\\', "/");
    let prefix = if prefix.is_empty() {
        prefix
    } else {
        format!("{prefix}/")
    };
    let commit = repo
        .revparse_single(reference)
        .and_then(|object| object.peel_to_commit())
        .map_err(failure)?;
    let tree = commit.tree().map_err(failure)?;
    let mut sources = BTreeMap::new();
    let mut total = 0u64;
    let mut errors = Vec::new();
    let mut examined = 0usize;
    tree.walk(git2::TreeWalkMode::PreOrder, |directory, entry| {
        examined += 1;
        if examined > 200_000 {
            return git2::TreeWalkResult::Abort;
        }
        let full = format!("{directory}{}", entry.name().unwrap_or(""));
        let Some(path) = full.strip_prefix(&prefix) else {
            return git2::TreeWalkResult::Ok;
        };
        if path.split('/').any(|part| {
            matches!(
                part,
                "node_modules"
                    | "vendor"
                    | "dist"
                    | "build"
                    | "coverage"
                    | "target"
                    | ".venv"
                    | "__pycache__"
            )
        }) {
            return git2::TreeWalkResult::Skip;
        }
        if path.split('/').count() > 64
            || sources.len() >= crate::indexing::MAX_FILES
            || total >= crate::indexing::MAX_TOTAL_BYTES
        {
            errors.push(Diagnostic {
                path: path.into(),
                line: 1,
                code: "baseline-limit".into(),
                message: "Baseline discovery limit reached".into(),
            });
            return git2::TreeWalkResult::Skip;
        }
        if entry.kind() != Some(git2::ObjectType::Blob) || entry.filemode() == 0o120000 {
            return git2::TreeWalkResult::Ok;
        }
        if parsing::language(path).is_none()
            && !path.ends_with(".json")
            && !path.ends_with(".gitignore")
            && !path.ends_with(".threadmarkignore")
        {
            return git2::TreeWalkResult::Ok;
        }
        match repo.find_blob(entry.id()) {
            Ok(blob) if blob.size() as u64 <= crate::indexing::MAX_FILE_BYTES => {
                total += blob.size() as u64;
                match std::str::from_utf8(blob.content()) {
                    Ok(source) => {
                        sources.insert(path.to_string(), source.to_string());
                    }
                    Err(error) => errors.push(Diagnostic {
                        path: path.into(),
                        line: 1,
                        code: "baseline-encoding".into(),
                        message: error.to_string(),
                    }),
                }
            }
            Ok(_) => errors.push(Diagnostic {
                path: path.into(),
                line: 1,
                code: "baseline-size".into(),
                message: "Baseline file exceeds size limit".into(),
            }),
            Err(error) => errors.push(Diagnostic {
                path: path.into(),
                line: 1,
                code: "baseline-read".into(),
                message: error.to_string(),
            }),
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(failure)?;
    let mut ignore = ignore::gitignore::GitignoreBuilder::new(root);
    for (path, source) in &sources {
        if path.ends_with(".gitignore") || path.ends_with(".threadmarkignore") {
            for line in source.lines() {
                if let Err(error) = ignore.add_line(Some(root.join(path)), line) {
                    errors.push(Diagnostic {
                        path: path.clone(),
                        line: 1,
                        code: "baseline-ignore".into(),
                        message: error.to_string(),
                    });
                }
            }
        }
    }
    let ignore = ignore.build().map_err(failure)?;
    let root_name = crate::indexing::root_string(root);
    let mut snapshot = Snapshot {
        schema_version: 1,
        root: root_name.clone(),
        name: root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        indexed_at: 0,
        nodes: vec![],
        edges: vec![],
        diagnostics: errors,
        project_files: vec![],
        languages: vec![],
        progress: Progress::default(),
    };
    let mut parsed = Vec::new();
    let mut reference_count = 0;
    for (path, source) in &sources {
        if ignore
            .matched_path_or_any_parents(root.join(path), false)
            .is_ignore()
        {
            continue;
        }
        if let Some(language) = parsing::language(path) {
            if !snapshot.languages.contains(&language.to_string()) {
                snapshot.languages.push(language.into());
            }
            match JavaScript.parse(&root_name, path, source) {
                Ok(file) => {
                    if snapshot.nodes.len() + file.nodes.len() > 200_000
                        || reference_count + file.references.len() + file.imports.len() > 500_000
                    {
                        snapshot.diagnostics.push(Diagnostic {
                            path: path.clone(),
                            line: 0,
                            code: "baseline-graph-limit".into(),
                            message:
                                "Baseline exceeds node/reference budgets; remaining files omitted"
                                    .into(),
                        });
                        break;
                    }
                    snapshot.nodes.extend(file.nodes.clone());
                    reference_count += file.references.len() + file.imports.len();
                    snapshot.diagnostics.extend(file.diagnostics.clone());
                    parsed.push(file);
                }
                Err(error) => snapshot.diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: 1,
                    code: "baseline-parse".into(),
                    message: error,
                }),
            }
        } else if path.ends_with(".json") {
            snapshot.project_files.push(path.clone());
        }
    }
    crate::resolution::resolve_sources(&|path| sources.get(path).cloned(), &parsed, &mut snapshot);
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().ok().map(str::to_string))
        .unwrap_or_else(|| "detached/unborn".into());
    Ok((commit.id().to_string(), branch, snapshot))
}
fn edge_key(edge: &Edge) -> (&str, &str, &str, &str) {
    (&edge.from, &edge.to, &edge.kind, &edge.confidence)
}
impl Engine {
    /// Read Git objects locally; never checks out files or runs Git hooks.
    pub fn compare(&self, reference: &str) -> Result<Comparison, Error> {
        let selected = self.snapshot(None)?;
        let root = crate::indexing::root(Path::new(&selected.root))?;
        let (commit, branch, before) = baseline(&root, reference)?;
        let current =
            self.index_repository_preserving_selection(&root, &AtomicBool::new(false), |_| {})?;
        let old: HashMap<_, _> = before
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let new: HashMap<_, _> = current
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let mut changes = Vec::new();
        let mut paths = HashSet::new();
        let mut current_targets = Vec::new();
        let mut removed_targets = Vec::new();
        for node in &current.nodes {
            let kind = match old.get(node.id.as_str()) {
                None => "added",
                Some(previous) if previous.fingerprint != node.fingerprint => "changed",
                _ => continue,
            };
            paths.insert(node.path.clone());
            current_targets.push(node.id.clone());
            if node.kind != "file" {
                changes.push(SymbolChange {
                    kind: kind.into(),
                    node: node.clone(),
                });
            }
        }
        for node in &before.nodes {
            if !new.contains_key(node.id.as_str()) {
                paths.insert(node.path.clone());
                removed_targets.push(node.id.clone());
                if node.kind != "file" {
                    changes.push(SymbolChange {
                        kind: "removed".into(),
                        node: node.clone(),
                    });
                }
            }
        }
        let old_edges: HashSet<_> = before.edges.iter().map(edge_key).collect();
        let new_edges: HashSet<_> = current.edges.iter().map(edge_key).collect();
        let added_relationship_count = current
            .edges
            .iter()
            .filter(|edge| !old_edges.contains(&edge_key(edge)))
            .count();
        let removed_relationship_count = before
            .edges
            .iter()
            .filter(|edge| !new_edges.contains(&edge_key(edge)))
            .count();
        let added = current
            .edges
            .iter()
            .filter(|edge| !old_edges.contains(&edge_key(edge)))
            .take(2000)
            .cloned()
            .collect();
        let removed = before
            .edges
            .iter()
            .filter(|edge| !new_edges.contains(&edge_key(edge)))
            .take(2000)
            .cloned()
            .collect();
        let diagnostics = before.diagnostics.clone();
        let current_impact = Graph::new(current).impact(&current_targets, 8, &[])?;
        let removed_impact = Graph::new(before).impact(&removed_targets, 8, &[])?;
        let mut changed_files: Vec<_> = paths.into_iter().collect();
        changed_files.sort();
        changes.sort_by_key(|change| (change.node.path.clone(), change.node.span.line));
        let symbol_change_count = changes.len();
        let truncated = symbol_change_count > 2000
            || added_relationship_count > 2000
            || removed_relationship_count > 2000
            || current_impact.truncated
            || removed_impact.truncated;
        changes.truncate(2000);
        Ok(Comparison {
            schema_version: 1,
            baseline: reference.into(),
            baseline_commit: commit,
            branch,
            changed_files,
            symbol_changes: changes,
            added_relationships: added,
            removed_relationships: removed,
            current_impact,
            removed_impact,
            diagnostics,
            symbol_change_count,
            added_relationship_count,
            removed_relationship_count,
            truncated,
        })
    }
}
