use crate::{
    Error,
    graph::Graph,
    model::{Edge, Node},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactItem {
    pub node: Node,
    pub hops: usize,
    pub evidence_path: Vec<Edge>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub schema_version: u32,
    pub targets: Vec<Node>,
    pub direct: Vec<ImpactItem>,
    pub transitive: Vec<ImpactItem>,
    pub possible: Vec<ImpactItem>,
    pub related_tests: Vec<ImpactItem>,
    pub truncated: bool,
    pub depth_limit: usize,
}

impl Graph {
    pub fn impact(
        &self,
        selectors: &[String],
        depth: usize,
        kinds: &[String],
    ) -> Result<Impact, Error> {
        let targets = selectors
            .iter()
            .take(2000)
            .map(|selector| self.select(selector).cloned())
            .collect::<Result<Vec<_>, _>>()?;
        let mut seeds: HashSet<_> = targets.iter().map(|node| node.id.clone()).collect();
        for target in &targets {
            if matches!(target.kind.as_str(), "file" | "class" | "interface") {
                for node in &self.snapshot.nodes {
                    if node.path == target.path
                        && (target.kind == "file"
                            || node
                                .qualified_name
                                .starts_with(&format!("{}.", target.qualified_name)))
                    {
                        seeds.insert(node.id.clone());
                    }
                }
            }
        }
        let depth = depth.clamp(1, 32);
        let mut queue = VecDeque::new();
        let mut distances = HashMap::new();
        let mut parents: HashMap<(String, bool), (String, bool, usize)> = HashMap::new();
        let mut ordered_seeds: Vec<_> = seeds.iter().collect();
        ordered_seeds.sort();
        for seed in ordered_seeds.into_iter().take(20_000) {
            distances.insert((seed.clone(), false), 0);
            queue.push_back((seed.clone(), false));
        }
        let mut truncated = selectors.len() > 2000 || seeds.len() > 20_000;
        let mut examined = 0;
        while let Some((id, possible)) = queue.pop_front() {
            let hops = distances[&(id.clone(), possible)];
            for index in self.incoming.get(&id).into_iter().flatten() {
                let edge = &self.snapshot.edges[*index];
                if edge.kind == "CONTAINS" || (!kinds.is_empty() && !kinds.contains(&edge.kind)) {
                    continue;
                }
                if seeds.contains(&edge.from) {
                    continue;
                }
                if hops >= depth {
                    truncated = true;
                    continue;
                }
                examined += 1;
                if examined > 150_000 || distances.len() >= 20_000 {
                    truncated = true;
                    break;
                }
                let next_possible =
                    possible || !matches!(edge.confidence.as_str(), "EXACT" | "RESOLVED");
                let key = (edge.from.clone(), next_possible);
                if !distances.contains_key(&key) {
                    distances.insert(key.clone(), hops + 1);
                    parents.insert(key.clone(), (id.clone(), possible, *index));
                    queue.push_back(key);
                }
            }
            if examined > 150_000 || distances.len() >= 20_000 {
                break;
            }
        }
        let mut output = Impact {
            schema_version: 1,
            targets,
            direct: Vec::new(),
            transitive: Vec::new(),
            possible: Vec::new(),
            related_tests: Vec::new(),
            truncated,
            depth_limit: depth,
        };
        let mut ids: Vec<_> = distances
            .keys()
            .map(|(id, _)| id.clone())
            .filter(|id| !seeds.contains(id))
            .collect();
        ids.sort();
        ids.dedup();
        if ids.len() > 2000 {
            output.truncated = true;
            ids.truncate(2000);
        }
        for id in ids {
            let possible = !distances.contains_key(&(id.clone(), false));
            let hops = distances[&(id.clone(), possible)];
            let Some(index) = self.indices.get(&id) else {
                continue;
            };
            let mut evidence = Vec::new();
            let mut state = (id.clone(), possible);
            while let Some((parent, parent_possible, index)) = parents.get(&state) {
                evidence.push(self.snapshot.edges[*index].clone());
                state = (parent.clone(), *parent_possible);
                if evidence.len() > depth {
                    break;
                }
            }
            let item = ImpactItem {
                node: self.snapshot.nodes[*index].clone(),
                hops,
                evidence_path: evidence,
            };
            if item.node.test || item.node.kind == "test" {
                output.related_tests.push(item.clone());
            }
            if possible {
                output.possible.push(item);
            } else if hops == 1 {
                output.direct.push(item);
            } else {
                output.transitive.push(item);
            }
        }
        for items in [
            &mut output.direct,
            &mut output.transitive,
            &mut output.possible,
            &mut output.related_tests,
        ] {
            items.sort_by_key(|item| (item.hops, item.node.path.clone(), item.node.span.line));
        }
        Ok(output)
    }
}
