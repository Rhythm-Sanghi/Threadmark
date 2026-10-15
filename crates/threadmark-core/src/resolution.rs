use crate::{indexing, model::*};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

#[derive(Default)]
struct Configuration {
    aliases: Vec<(String, String, Vec<String>)>,
    packages: Vec<(String, String, serde_json::Value)>,
}

fn normalize(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => (),
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}
fn directory(path: &str) -> &str {
    path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}
fn join(base: &str, path: &str) -> Option<String> {
    normalize(&format!("{base}/{path}"))
}

#[derive(Default)]
struct CompilerOptions {
    base: Option<String>,
    paths: Option<(String, BTreeMap<String, Vec<String>>)>,
}

fn compiler_options(
    path: &str,
    value: &serde_json::Value,
    reader: &impl Fn(&str) -> Option<String>,
    stack: &mut Vec<String>,
    budget: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> CompilerOptions {
    if stack.len() >= 16 || stack.iter().any(|parent| parent == path) {
        diagnostics.push(Diagnostic {
            path: path.into(),
            line: 0,
            code: "config-extends".into(),
            message: "Configuration inheritance contains a cycle or exceeds 16 levels".into(),
        });
        return CompilerOptions::default();
    }
    stack.push(path.into());
    *budget += 1;
    let mut options = CompilerOptions::default();
    let parents: Vec<_> = match value.get("extends") {
        Some(serde_json::Value::String(parent)) => vec![parent.as_str()],
        Some(serde_json::Value::Array(parents)) => parents
            .iter()
            .filter_map(|parent| parent.as_str())
            .collect(),
        _ => vec![],
    };
    for parent in parents.into_iter().take(16) {
        if *budget >= 64 {
            diagnostics.push(Diagnostic {
                path: path.into(),
                line: 0,
                code: "config-extends".into(),
                message: "Inheritance traversal exceeds the 64-configuration budget".into(),
            });
            break;
        }
        let loaded = if parent.starts_with('.') {
            join(directory(path), parent).and_then(|relative| {
                [
                    relative.clone(),
                    format!("{relative}.json"),
                    format!("{relative}/tsconfig.json"),
                ]
                .into_iter()
                .find_map(|candidate| reader(&candidate).map(|source| (candidate, source)))
            })
        } else {
            None
        };
        if let Some((parent_path, source)) = loaded {
            match json5::from_str::<serde_json::Value>(&source) {
                Ok(value) => {
                    let inherited =
                        compiler_options(&parent_path, &value, reader, stack, budget, diagnostics);
                    if inherited.base.is_some() {
                        options.base = inherited.base;
                    }
                    if inherited.paths.is_some() {
                        options.paths = inherited.paths;
                    }
                }
                Err(error) => diagnostics.push(Diagnostic {
                    path: parent_path,
                    line: 0,
                    code: "configuration-error".into(),
                    message: error.to_string(),
                }),
            }
        } else {
            diagnostics.push(Diagnostic {path:path.into(),line:0,code:"config-extends".into(),message:format!("Cannot load inherited configuration {parent}; only readable repository-local relative paths are supported")});
        }
    }
    if value["extends"]
        .as_array()
        .is_some_and(|parents| parents.len() > 16)
    {
        diagnostics.push(Diagnostic {
            path: path.into(),
            line: 0,
            code: "config-extends".into(),
            message: "At most 16 inherited configurations are loaded".into(),
        });
    }
    if let Some(base) = value["compilerOptions"]["baseUrl"].as_str() {
        options.base = join(directory(path), base);
    }
    if let Some(paths) = value["compilerOptions"]["paths"].as_object() {
        options.paths = Some((
            directory(path).into(),
            paths
                .iter()
                .filter_map(|(pattern, targets)| {
                    targets.as_array().map(|targets| {
                        (
                            pattern.clone(),
                            targets
                                .iter()
                                .filter_map(|target| target.as_str().map(str::to_owned))
                                .collect(),
                        )
                    })
                })
                .collect(),
        ));
    }
    stack.pop();
    options
}

fn config(snapshot: &mut Snapshot, reader: &impl Fn(&str) -> Option<String>) -> Configuration {
    let mut configuration = Configuration::default();
    for path in snapshot.project_files.clone() {
        if !path.ends_with("package.json")
            && !path.ends_with("tsconfig.json")
            && !path.ends_with("jsconfig.json")
        {
            continue;
        }
        let source = match reader(&path) {
            Some(source) => source,
            None => continue,
        };
        let value: serde_json::Value = match json5::from_str(&source) {
            Ok(value) => value,
            Err(error) => {
                snapshot.diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: 0,
                    code: "configuration-error".into(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        if path.ends_with("package.json") {
            if let Some(name) = value["name"].as_str() {
                configuration
                    .packages
                    .push((name.into(), directory(&path).into(), value));
            }
        } else if path.ends_with("tsconfig.json") || path.ends_with("jsconfig.json") {
            let options = compiler_options(
                &path,
                &value,
                reader,
                &mut Vec::new(),
                &mut 0,
                &mut snapshot.diagnostics,
            );
            if let Some((origin, paths)) = options.paths {
                let base = options.base.as_deref().unwrap_or(&origin);
                for (pattern, targets) in paths {
                    configuration.aliases.push((
                        directory(&path).into(),
                        pattern,
                        targets
                            .iter()
                            .filter_map(|target| join(base, target))
                            .collect(),
                    ));
                }
            }
        }
    }
    configuration
        .aliases
        .sort_by_key(|a| std::cmp::Reverse(a.0.len()));
    configuration
}

fn wildcard(pattern: &str, value: &str) -> Option<String> {
    match pattern.split_once('*') {
        Some((before, after))
            if value.starts_with(before)
                && value.ends_with(after)
                && value.len() >= before.len() + after.len() =>
        {
            Some(value[before.len()..value.len() - after.len()].into())
        }
        None if pattern == value => Some(String::new()),
        _ => None,
    }
}

fn conditional(value: &serde_json::Value) -> Option<&str> {
    if let Some(value) = value.as_str() {
        return Some(value);
    }
    for key in ["types", "import", "default", "require"] {
        if let Some(value) = value.get(key).and_then(conditional) {
            return Some(value);
        }
    }
    None
}

fn file_candidate(path: &str, paths: &HashSet<String>) -> Option<String> {
    if paths.contains(path) {
        return Some(path.into());
    }
    if let Some(stem) = path.strip_suffix(".js") {
        for suffix in [".ts", ".tsx"] {
            let candidate = format!("{stem}{suffix}");
            if paths.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    for suffix in [
        ".ts",
        ".tsx",
        ".mts",
        ".cts",
        ".js",
        ".jsx",
        ".mjs",
        ".cjs",
        "/index.ts",
        "/index.tsx",
        "/index.js",
        "/index.jsx",
    ] {
        let candidate = format!("{path}{suffix}");
        if paths.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn resolve_import(
    file: &str,
    specifier: &str,
    paths: &HashSet<String>,
    config: &Configuration,
) -> Result<String, String> {
    if let Some(module) = specifier.strip_prefix("py:") {
        let dots = module
            .chars()
            .take_while(|character| *character == '.')
            .count();
        if dots
            > directory(file)
                .split('/')
                .filter(|part| !part.is_empty())
                .count()
                + 1
        {
            return Err("Relative Python import escapes indexed package root".into());
        }
        let module = module[dots..].replace('.', "/");
        let bases = if dots > 0 {
            let mut base = directory(file).to_string();
            for _ in 1..dots {
                base = directory(&base).to_string();
            }
            vec![if module.is_empty() {
                base
            } else if base.is_empty() {
                module
            } else {
                format!("{base}/{module}")
            }]
        } else {
            vec![module.clone(), format!("src/{module}")]
        };
        let mut matches = Vec::new();
        for base in bases {
            for candidate in [format!("{base}.py"), format!("{base}/__init__.py")] {
                if paths.contains(&candidate) {
                    matches.push(candidate);
                }
            }
        }
        matches.sort();
        matches.dedup();
        return if matches.len() == 1 {
            Ok(matches.remove(0))
        } else {
            Err(format!(
                "Python module {specifier} has {} indexed candidates; runtime import paths are not executed",
                matches.len()
            ))
        };
    }
    if specifier.starts_with('.') {
        return join(directory(file), specifier)
            .and_then(|path| file_candidate(&path, paths))
            .ok_or_else(|| format!("No indexed source matches {specifier}"));
    }
    let applicable: Vec<_> = config
        .aliases
        .iter()
        .filter(|(base, _, _)| base.is_empty() || file.starts_with(&format!("{base}/")))
        .collect();
    let closest = applicable.first().map(|(base, _, _)| base.len());
    for (base, pattern, targets) in applicable {
        if Some(base.len()) != closest {
            continue;
        }
        if let Some(replacement) = wildcard(pattern, specifier) {
            let candidates: HashSet<_> = targets
                .iter()
                .filter_map(|target| file_candidate(&target.replace('*', &replacement), paths))
                .collect();
            if candidates.len() == 1 {
                return Ok(candidates.into_iter().next().unwrap());
            }
            if candidates.len() > 1 {
                return Err(format!("Ambiguous path alias {specifier}"));
            }
        }
    }
    for (name, base, package) in &config.packages {
        let subpath = if name == specifier {
            ""
        } else if let Some(path) = specifier.strip_prefix(&format!("{name}/")) {
            path
        } else {
            continue;
        };
        let export_key = if subpath.is_empty() {
            ".".into()
        } else {
            format!("./{subpath}")
        };
        let exports = &package["exports"];
        let mut target = exports
            .get(&export_key)
            .and_then(conditional)
            .map(str::to_owned);
        if target.is_none() && subpath.is_empty() {
            target = conditional(exports).map(str::to_owned);
        }
        if target.is_none()
            && let Some(exports) = exports.as_object()
        {
            for (key, value) in exports {
                if let Some(replacement) = wildcard(key, &export_key) {
                    target = conditional(value).map(|value| value.replace('*', &replacement));
                }
            }
        }
        if target.is_none() && exports.is_null() {
            target = if subpath.is_empty() {
                ["types", "module", "main"]
                    .iter()
                    .find_map(|key| package[key].as_str())
                    .map(str::to_owned)
                    .or(Some("index".into()))
            } else {
                Some(subpath.into())
            };
        }
        return target
            .and_then(|target| join(base, &target))
            .and_then(|path| file_candidate(&path, paths))
            .ok_or_else(|| {
                format!("Workspace package {name} has no indexed target for {specifier}")
            });
    }
    Err(format!(
        "External package or unresolved alias {specifier}; no local source target"
    ))
}

pub fn resolve(root: &Path, files: &[ParsedFile], snapshot: &mut Snapshot) {
    resolve_sources(
        &|path| indexing::safe_read(root, path, indexing::MAX_FILE_BYTES).ok(),
        files,
        snapshot,
    );
}

pub fn resolve_sources(
    reader: &impl Fn(&str) -> Option<String>,
    files: &[ParsedFile],
    snapshot: &mut Snapshot,
) {
    let config = config(snapshot, reader);
    let mut seen_configuration = HashSet::new();
    snapshot.nodes.retain(|node| {
        node.kind != "configuration_key" || seen_configuration.insert(node.id.clone())
    });
    let paths: HashSet<_> = files.iter().map(|file| file.path.clone()).collect();
    let nodes: HashMap<_, _> = snapshot
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let file_nodes: HashMap<_, _> = files
        .iter()
        .map(|file| (file.path.clone(), &file.nodes[0]))
        .collect();
    let mut exports: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    for file in files {
        let mut names = BTreeMap::new();
        let declarations: HashMap<_, _> = file
            .nodes
            .iter()
            .map(|node| (node.qualified_name.as_str(), node))
            .collect();
        for (alias, name) in &file.exports {
            if let Some(node) = declarations.get(name.as_str()) {
                names.insert(alias.clone(), node.id.clone());
            }
        }
        exports.insert(file.path.clone(), names);
        for node in file.nodes.iter().skip(1) {
            if node.kind == "configuration_key" {
                continue;
            }
            let parent = node
                .qualified_name
                .rsplit_once('.')
                .and_then(|(parent, _)| declarations.get(parent).copied())
                .unwrap_or(&file.nodes[0]);
            push_edge(
                &mut snapshot.edges,
                &mut snapshot.diagnostics,
                &mut snapshot.progress,
                edge(
                    parent,
                    node,
                    "CONTAINS",
                    "EXACT",
                    &node.span,
                    &format!("{} contains {}", parent.qualified_name, node.qualified_name),
                ),
            );
        }
    }
    let mut resolved = HashMap::new();
    // A target lookup must not scan every source file for every reference.
    let ambiguous_nodes: HashSet<_> = files
        .iter()
        .flat_map(|file| file.ambiguous.iter())
        .collect();
    let reassigned_nodes: HashSet<_> = files
        .iter()
        .flat_map(|file| {
            let changed: HashSet<_> = file.reassigned.iter().map(|name| name.as_str()).collect();
            file.nodes
                .iter()
                .filter(move |node| changed.contains(node.qualified_name.as_str()))
                .map(|node| &node.id)
        })
        .collect();
    for file in files {
        for (index, import) in file.imports.iter().enumerate() {
            match resolve_import(&file.path, &import.specifier, &paths, &config) {
                Ok(target) => {
                    push_edge(
                        &mut snapshot.edges,
                        &mut snapshot.diagnostics,
                        &mut snapshot.progress,
                        edge(
                            &file.nodes[0],
                            file_nodes[&target],
                            "IMPORTS",
                            "EXACT",
                            &import.span,
                            &import.evidence,
                        ),
                    );
                    resolved.insert((file.path.clone(), index), target);
                }
                Err(message) => snapshot.diagnostics.push(Diagnostic {
                    path: file.path.clone(),
                    line: import.span.line,
                    code: "unresolved-import".into(),
                    message,
                }),
            }
        }
    }
    // A bounded fixed point resolves chains of barrels, including circular
    // re-exports. Ambiguous duplicate exports are removed rather than guessed.
    // Explicit local/named exports take precedence over export-star entries,
    // even when an explicit export cannot itself be resolved.
    let explicit_exports: HashSet<_> = files
        .iter()
        .filter(|file| file.nodes[0].language != "python")
        .flat_map(|file| {
            file.exports
                .iter()
                .map(|(alias, _)| (file.path.as_str(), alias.as_str()))
                .chain(
                    file.imports
                        .iter()
                        .filter(|import| import.reexport)
                        .flat_map(|import| {
                            import
                                .bindings
                                .iter()
                                .map(|(alias, _)| (file.path.as_str(), alias.as_str()))
                        }),
                )
        })
        .collect();
    let mut ambiguous = HashSet::new();
    for _ in 0..files.len().min(256) {
        let mut updates = Vec::new();
        for file in files {
            for (index, import) in file
                .imports
                .iter()
                .enumerate()
                .filter(|(_, import)| import.reexport)
            {
                if let Some(target) = resolved.get(&(file.path.clone(), index)) {
                    for (alias, id) in &exports[target] {
                        if alias == "default" && import.bindings.is_empty() {
                            continue;
                        }
                        if import.bindings.is_empty() {
                            if explicit_exports.contains(&(file.path.as_str(), alias.as_str())) {
                                continue;
                            }
                            updates.push((file.path.clone(), alias.clone(), id.clone()));
                        } else {
                            for (new_name, old_name) in &import.bindings {
                                if alias == old_name {
                                    updates.push((file.path.clone(), new_name.clone(), id.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut changed = false;
        for (file, alias, id) in updates {
            if ambiguous.contains(&(file.clone(), alias.clone())) {
                continue;
            }
            match exports[&file].get(&alias) {
                Some(existing) if existing != &id => {
                    exports.get_mut(&file).unwrap().remove(&alias);
                    ambiguous.insert((file, alias));
                    changed = true;
                }
                None => {
                    exports.get_mut(&file).unwrap().insert(alias, id);
                    changed = true;
                }
                _ => (),
            }
        }
        if !changed {
            break;
        }
    }
    // A barrel may have propagated an apparently unique target before another
    // chain exposed its ambiguity. Retract that target through downstream
    // re-exports as well; otherwise consumers can retain a confident stale edge.
    let mut importers: HashMap<&str, Vec<(&ParsedFile, &Import)>> = HashMap::new();
    for file in files {
        for (index, import) in file
            .imports
            .iter()
            .enumerate()
            .filter(|(_, import)| import.reexport)
        {
            if let Some(target) = resolved.get(&(file.path.clone(), index)) {
                importers
                    .entry(target.as_str())
                    .or_default()
                    .push((file, import));
            }
        }
    }
    let mut pending: std::collections::VecDeque<_> = ambiguous.iter().cloned().collect();
    while let Some((target, alias)) = pending.pop_front() {
        for (file, import) in importers.get(target.as_str()).into_iter().flatten() {
            let aliases: Vec<_> = if import.bindings.is_empty() {
                if alias == "default"
                    || explicit_exports.contains(&(file.path.as_str(), alias.as_str()))
                {
                    continue;
                }
                vec![alias.as_str()]
            } else {
                import
                    .bindings
                    .iter()
                    .filter(|(_, old)| old == &alias)
                    .map(|(new, _)| new.as_str())
                    .collect()
            };
            for alias in aliases {
                let key = (file.path.clone(), alias.to_owned());
                if ambiguous.insert(key.clone()) {
                    exports.get_mut(&file.path).unwrap().remove(alias);
                    pending.push_back(key);
                }
            }
        }
    }
    for (file, alias) in ambiguous {
        snapshot.diagnostics.push(Diagnostic {
            path: file,
            line: 0,
            code: "ambiguous-export".into(),
            message: format!("Multiple targets export {alias}"),
        });
    }
    for file in files {
        for id in exports[&file.path].values() {
            if let Some(node) = nodes.get(id) {
                push_edge(
                    &mut snapshot.edges,
                    &mut snapshot.diagnostics,
                    &mut snapshot.progress,
                    edge(
                        &file.nodes[0],
                        node,
                        "EXPORTS",
                        "EXACT",
                        &node.span,
                        &format!("Exports {}", node.name),
                    ),
                );
            }
        }
        let mut bindings = HashMap::new();
        let mut namespaces = HashMap::new();
        let reassigned: HashSet<_> = file.reassigned.iter().map(|name| name.as_str()).collect();
        let mut locals = HashMap::new();
        for node in file.nodes.iter().filter(|node| node.kind != "file") {
            locals
                .entry(node.qualified_name.as_str())
                .and_modify(|entry| *entry = None)
                .or_insert(Some(node));
        }
        for (index, import) in file.imports.iter().enumerate() {
            if import.reexport {
                continue;
            }
            if let Some(target) = resolved.get(&(file.path.clone(), index)) {
                for (local, remote) in &import.bindings {
                    if remote == "*" {
                        namespaces.insert(local.clone(), target.clone());
                    } else if let Some(id) = exports[target].get(remote) {
                        bindings.insert(local.clone(), id.clone());
                        push_edge(
                            &mut snapshot.edges,
                            &mut snapshot.diagnostics,
                            &mut snapshot.progress,
                            edge(
                                &file.nodes[0],
                                nodes[id],
                                if import.type_only {
                                    "USES_TYPE"
                                } else {
                                    "REFERENCES"
                                },
                                "RESOLVED",
                                &import.span,
                                &import.evidence,
                            ),
                        );
                    } else {
                        snapshot.diagnostics.push(Diagnostic {
                            path: file.path.clone(),
                            line: import.span.line,
                            code: "unresolved-export".into(),
                            message: format!(
                                "{} does not have a resolved export named {remote}",
                                import.specifier
                            ),
                        });
                    }
                }
            }
        }
        for reference in &file.references {
            if file.ambiguous.contains(&reference.owner) {
                continue;
            }
            if reference.shadowed {
                snapshot.diagnostics.push(Diagnostic {
                    path: file.path.clone(),
                    line: reference.span.line,
                    code: "unresolved-call".into(),
                    message: format!(
                        "{} is shadowed by a parameter; its runtime value is unknown",
                        reference.name
                    ),
                });
                continue;
            }
            let Some(owner) = nodes.get(&reference.owner) else {
                continue;
            };
            let mut target = None;
            let mut rebound = false;
            let mut current_scope = if owner.kind == "file" {
                String::new()
            } else {
                owner.qualified_name.clone()
            };
            loop {
                let qualified = if current_scope.is_empty() {
                    reference.name.clone()
                } else {
                    format!("{current_scope}.{}", reference.name)
                };
                if reference.kind != "USES_TYPE" && reassigned.contains(qualified.as_str()) {
                    rebound = true;
                    break;
                }
                if let Some(Some(candidate)) = locals.get(qualified.as_str()) {
                    target = Some(candidate.id.clone());
                    break;
                }
                if current_scope.is_empty() {
                    break;
                }
                current_scope = current_scope
                    .rsplit_once('.')
                    .map(|(scope, _)| scope.to_owned())
                    .unwrap_or_default();
            }
            if rebound {
                continue;
            }
            if target.is_none() {
                target = bindings.get(&reference.name).cloned();
            }
            if target.is_none()
                && let Some((namespace, member)) = reference.name.split_once('.')
            {
                target = namespaces
                    .get(namespace)
                    .and_then(|path| exports[path].get(member))
                    .cloned();
            }
            if let Some(target) = target.filter(|target| {
                !ambiguous_nodes.contains(target)
                    && (reference.kind == "USES_TYPE" || !reassigned_nodes.contains(target))
            }) {
                if let Some(target) = nodes.get(&target) {
                    if reference.kind != "CALLS"
                        || matches!(target.kind.as_str(), "function" | "method" | "class")
                    {
                        push_edge(
                            &mut snapshot.edges,
                            &mut snapshot.diagnostics,
                            &mut snapshot.progress,
                            edge(
                                owner,
                                target,
                                &reference.kind,
                                "RESOLVED",
                                &reference.span,
                                &reference.evidence,
                            ),
                        );
                        if owner.test && reference.kind == "CALLS" {
                            push_edge(
                                &mut snapshot.edges,
                                &mut snapshot.diagnostics,
                                &mut snapshot.progress,
                                edge(
                                    owner,
                                    target,
                                    "TESTS",
                                    "RESOLVED",
                                    &reference.span,
                                    &reference.evidence,
                                ),
                            );
                        }
                    } else {
                        snapshot.diagnostics.push(Diagnostic {
                            path: file.path.clone(),
                            line: reference.span.line,
                            code: "unresolved-call".into(),
                            message: format!(
                                "{} is a variable with unknown call target",
                                reference.name
                            ),
                        });
                    }
                }
            } else if reference.kind == "CALLS" {
                snapshot.diagnostics.push(Diagnostic {
                    path: file.path.clone(),
                    line: reference.span.line,
                    code: "unresolved-call".into(),
                    message: format!("No unambiguous static target for {}", reference.name),
                });
            }
        }
    }
    snapshot.edges.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.edges.dedup_by(|a, b| a.id == b.id);
}
