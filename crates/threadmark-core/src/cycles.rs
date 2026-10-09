use crate::{
    graph::{Graph, Slice},
    model::*,
};
use std::collections::{HashMap, HashSet};
fn components(snapshot: &Snapshot) -> Vec<Vec<String>> {
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    for edge in snapshot.edges.iter().filter(|edge| edge.kind == "IMPORTS") {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    for node in snapshot.nodes.iter().filter(|node| node.kind == "file") {
        let mut stack = vec![(node.id.clone(), false)];
        while let Some((id, done)) = stack.pop() {
            if done {
                order.push(id);
                continue;
            }
            if visited.insert(id.clone()) {
                stack.push((id.clone(), true));
                for next in outgoing.get(&id).into_iter().flatten() {
                    if !visited.contains(next) {
                        stack.push((next.clone(), false));
                    }
                }
            }
        }
    }
    visited.clear();
    let mut components = Vec::new();
    for id in order.into_iter().rev() {
        if visited.contains(&id) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            if visited.insert(id.clone()) {
                component.push(id.clone());
                stack.extend(
                    incoming
                        .get(&id)
                        .into_iter()
                        .flatten()
                        .filter(|next| !visited.contains(*next))
                        .cloned(),
                );
            }
        }
        if component.len() > 1
            || outgoing
                .get(&component[0])
                .is_some_and(|neighbors| neighbors.contains(&component[0]))
        {
            components.push(component);
        }
    }
    components
}
pub fn diagnostics(snapshot: &Snapshot) -> Vec<Diagnostic> {
    let nodes: HashMap<_, _> = snapshot
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    components(snapshot).into_iter().take(100).map(|component|{let mut paths:Vec<_>=component.iter().filter_map(|id|nodes.get(id.as_str()).map(|node|node.path.clone())).collect();paths.sort();Diagnostic{path:paths.first().cloned().unwrap_or_default(),line:1,code:"circular-dependency".into(),message:format!("{} files have mutually reachable local imports: {}. Inspect with threadmark cycles.",paths.len(),paths.iter().take(20).cloned().collect::<Vec<_>>().join(", "))}}).collect()
}
impl Graph {
    pub fn cycles(&self) -> Vec<Slice> {
        components(&self.snapshot)
            .into_iter()
            .take(100)
            .map(|mut component| {
                let truncated = component.len() > 120;
                component.truncate(120);
                let ids: HashSet<_> = component.iter().cloned().collect();
                Slice {
                    schema_version: 1,
                    focus: component[0].clone(),
                    nodes: component
                        .iter()
                        .filter_map(|id| {
                            self.indices
                                .get(id)
                                .map(|index| self.snapshot.nodes[*index].clone())
                        })
                        .collect(),
                    edges: self
                        .snapshot
                        .edges
                        .iter()
                        .filter(|edge| {
                            edge.kind == "IMPORTS"
                                && ids.contains(&edge.from)
                                && ids.contains(&edge.to)
                        })
                        .cloned()
                        .collect(),
                    truncated,
                }
            })
            .collect()
    }
}
