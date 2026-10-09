use crate::{Error, model::*};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slice {
    pub schema_version: u32,
    pub focus: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub schema_version: u32,
    pub root: String,
    pub name: String,
    pub indexed_at: u64,
    pub source_files: usize,
    pub symbols: usize,
    pub relationships: usize,
    pub diagnostics: usize,
    pub languages: Vec<String>,
    pub project_files: Vec<String>,
    pub progress: Progress,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub name: String,
    pub directory: bool,
    pub node_id: Option<String>,
}

pub struct Graph {
    pub snapshot: Snapshot,
    pub indices: HashMap<String, usize>,
    pub outgoing: HashMap<String, Vec<usize>>,
    pub incoming: HashMap<String, Vec<usize>>,
}

impl Graph {
    pub fn new(snapshot: Snapshot) -> Self {
        let indices = snapshot
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect();
        let mut outgoing: HashMap<String, Vec<usize>> = HashMap::new();
        let mut incoming: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, edge) in snapshot.edges.iter().enumerate() {
            outgoing.entry(edge.from.clone()).or_default().push(index);
            incoming.entry(edge.to.clone()).or_default().push(index);
        }
        Self {
            snapshot,
            indices,
            outgoing,
            incoming,
        }
    }

    pub fn overview(&self) -> Overview {
        let snapshot = &self.snapshot;
        Overview {
            schema_version: 1,
            root: snapshot.root.clone(),
            name: snapshot.name.clone(),
            indexed_at: snapshot.indexed_at,
            source_files: snapshot.progress.source_files,
            symbols: snapshot.progress.symbols,
            relationships: snapshot.edges.len(),
            diagnostics: snapshot.diagnostics.len(),
            languages: snapshot.languages.clone(),
            project_files: snapshot.project_files.clone(),
            progress: snapshot.progress.clone(),
        }
    }

    pub fn select(&self, selector: &str) -> Result<&Node, Error> {
        if let Some(index) = self.indices.get(selector) {
            return Ok(&self.snapshot.nodes[*index]);
        }
        let matches: Vec<_> = self
            .snapshot
            .nodes
            .iter()
            .filter(|node| {
                node.name == selector
                    || node.qualified_name == selector
                    || (node.kind == "file" && node.path == selector)
                    || format!("{}#{}", node.path, node.qualified_name) == selector
            })
            .collect();
        match matches.as_slice() {
            [node] => Ok(node),
            [] => Err(Error::Analysis(format!(
                "No indexed file or symbol matches {selector:?}"
            ))),
            _ => Err(Error::Analysis(format!(
                "Ambiguous selector {selector:?}. Use a node ID or path#qualified_name. Matches: {}",
                matches
                    .iter()
                    .take(10)
                    .map(|node| format!("{}#{}", node.path, node.qualified_name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Node> {
        let query = query.to_lowercase();
        let mut matches: Vec<_> = self
            .snapshot
            .nodes
            .iter()
            .filter(|node| {
                node.name.to_lowercase().contains(&query)
                    || node.path.to_lowercase().contains(&query)
                    || node.qualified_name.to_lowercase().contains(&query)
            })
            .collect();
        matches.sort_by_key(|node| {
            (
                node.name.to_lowercase() != query,
                node.kind != "file",
                node.path.clone(),
                node.span.line,
            )
        });
        matches.into_iter().take(limit.min(100)).cloned().collect()
    }

    pub fn tree(&self, directory: &str) -> Vec<TreeEntry> {
        let prefix = if directory.is_empty() {
            String::new()
        } else {
            format!("{directory}/")
        };
        let mut entries = std::collections::BTreeMap::new();
        for node in self
            .snapshot
            .nodes
            .iter()
            .filter(|node| node.kind == "file")
        {
            if let Some(rest) = node.path.strip_prefix(&prefix) {
                let name = rest.split('/').next().unwrap_or(rest);
                let is_directory = rest.contains('/');
                let path = format!("{prefix}{name}");
                entries.entry(path.clone()).or_insert_with(|| TreeEntry {
                    path,
                    name: name.into(),
                    directory: is_directory,
                    node_id: if is_directory {
                        None
                    } else {
                        Some(node.id.clone())
                    },
                });
            }
        }
        let mut values: Vec<_> = entries.into_values().collect();
        values.sort_by_key(|entry| (!entry.directory, entry.name.clone()));
        values.truncate(1000);
        values
    }

    pub fn slice(
        &self,
        selector: &str,
        direction: &str,
        depth: usize,
        limit: usize,
        kinds: &[String],
    ) -> Result<Slice, Error> {
        let focus = self.select(selector)?.id.clone();
        if !matches!(direction, "dependencies" | "dependents" | "both") {
            return Err(Error::Analysis(
                "Direction must be dependencies, dependents or both".into(),
            ));
        }
        let limit = limit.clamp(1, 120);
        let mut visited = HashSet::from([focus.clone()]);
        let mut selected_edges = HashSet::new();
        let mut queue = VecDeque::from([(focus.clone(), 0)]);
        let mut truncated = false;
        while let Some((id, hops)) = queue.pop_front() {
            if hops >= depth.min(3) {
                continue;
            }
            let outgoing = self
                .outgoing
                .get(&id)
                .into_iter()
                .flatten()
                .copied()
                .filter(|_| direction != "dependents");
            let incoming = self
                .incoming
                .get(&id)
                .into_iter()
                .flatten()
                .copied()
                .filter(|_| direction != "dependencies");
            for index in outgoing.chain(incoming) {
                let edge = &self.snapshot.edges[index];
                if edge.kind == "CONTAINS" && kinds.is_empty() {
                    continue;
                }
                if !kinds.is_empty() && !kinds.contains(&edge.kind) {
                    continue;
                }
                let other = if edge.from == id {
                    &edge.to
                } else {
                    &edge.from
                };
                if !visited.contains(other) && visited.len() >= limit {
                    truncated = true;
                    continue;
                }
                if !self.indices.contains_key(other) {
                    continue;
                }
                selected_edges.insert(index);
                if visited.insert(other.clone()) {
                    queue.push_back((other.clone(), hops + 1));
                }
            }
        }
        let mut node_ids: Vec<_> = visited.into_iter().collect();
        node_ids.sort();
        let mut edge_indices: Vec<_> = selected_edges.into_iter().collect();
        edge_indices.sort();
        Ok(Slice {
            schema_version: 1,
            focus,
            nodes: node_ids
                .iter()
                .map(|id| self.snapshot.nodes[self.indices[id]].clone())
                .collect(),
            edges: edge_indices
                .iter()
                .map(|index| self.snapshot.edges[*index].clone())
                .collect(),
            truncated,
        })
    }

    pub fn find_path(&self, from: &str, to: &str, max_depth: usize) -> Result<Slice, Error> {
        let start = self.select(from)?.id.clone();
        let target = self.select(to)?.id.clone();
        let mut queue = VecDeque::from([(start.clone(), 0)]);
        let mut parents = HashMap::new();
        let mut seen = HashSet::from([start.clone()]);
        let mut truncated = false;
        while let Some((id, hops)) = queue.pop_front() {
            if id == target {
                break;
            }
            if hops >= max_depth.min(32) {
                truncated = true;
                continue;
            }
            for index in self.outgoing.get(&id).into_iter().flatten() {
                let edge = &self.snapshot.edges[*index];
                if edge.kind == "CONTAINS" {
                    continue;
                }
                if seen.len() >= 10_000 {
                    truncated = true;
                    break;
                }
                if seen.insert(edge.to.clone()) {
                    parents.insert(edge.to.clone(), *index);
                    queue.push_back((edge.to.clone(), hops + 1));
                }
            }
        }
        if !seen.contains(&target) {
            return Ok(Slice {
                schema_version: 1,
                focus: start,
                nodes: Vec::new(),
                edges: Vec::new(),
                truncated,
            });
        }
        let mut nodes = vec![self.snapshot.nodes[self.indices[&target]].clone()];
        let mut edges = Vec::new();
        let mut current = target;
        while current != start {
            let edge = &self.snapshot.edges[parents[&current]];
            edges.push(edge.clone());
            current = edge.from.clone();
            nodes.push(self.snapshot.nodes[self.indices[&current]].clone());
        }
        nodes.reverse();
        edges.reverse();
        Ok(Slice {
            schema_version: 1,
            focus: start,
            nodes,
            edges,
            truncated: false,
        })
    }
}
