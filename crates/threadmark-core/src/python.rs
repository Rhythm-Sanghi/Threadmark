//! Python adapter. Syntax only: never invokes Python or import hooks.
use crate::{
    model::*,
    parsing::{LanguageAdapter, span},
};
use tree_sitter::{Node as Syntax, Parser};
pub struct Python;
fn text<'a>(node: Syntax<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
fn children(node: Syntax<'_>) -> Vec<Syntax<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
fn field<'a>(node: Syntax<'a>, name: &str) -> Option<Syntax<'a>> {
    node.child_by_field_name(name)
}
fn binding(node: Syntax<'_>, source: &str) -> (String, String) {
    if node.kind() == "aliased_import" {
        let name = field(node, "name")
            .map(|node| text(node, source))
            .unwrap_or("");
        let alias = field(node, "alias")
            .map(|node| text(node, source))
            .unwrap_or(name);
        (alias.into(), name.into())
    } else {
        let name = text(node, source);
        (name.into(), name.into())
    }
}
fn shadowed(mut node: Syntax<'_>, name: &str, source: &str) -> bool {
    let name = name.split('.').next().unwrap_or(name);
    while let Some(parent) = node.parent() {
        if let Some(parameters) = field(parent, "parameters") {
            let mut stack = children(parameters);
            while let Some(parameter) = stack.pop() {
                if parameter.kind() == "identifier" && text(parameter, source) == name {
                    return true;
                }
                if !matches!(parameter.kind(), "type" | "type_annotation") {
                    stack.extend(children(parameter));
                }
            }
        }
        node = parent;
    }
    false
}
impl LanguageAdapter for Python {
    fn parse(&self, root: &str, path: &str, source: &str) -> Result<ParsedFile, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|error| error.to_string())?;
        let started = std::time::Instant::now();
        let mut timeout = |_: &tree_sitter::ParseState| {
            if started.elapsed() > std::time::Duration::from_secs(2) {
                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        };
        let tree = parser
            .parse_with_options(
                &mut |offset, _| &source.as_bytes()[offset.min(source.len())..],
                None,
                Some(tree_sitter::ParseOptions::new().progress_callback(&mut timeout)),
            )
            .ok_or("Python parser exceeded two seconds")?;
        let hash = blake3::hash(source.as_bytes()).to_hex().to_string();
        let file = Node {
            id: stable_id(root, path, "python", path, "file"),
            name: path.rsplit('/').next().unwrap_or(path).into(),
            qualified_name: path.into(),
            kind: "file".into(),
            path: path.into(),
            language: "python".into(),
            span: span(tree.root_node()),
            exported: false,
            test: path
                .rsplit('/')
                .next()
                .is_some_and(|name| name.starts_with("test_") || name.ends_with("_test.py"))
                || path.starts_with("tests/"),
            fingerprint: fingerprint(source),
        };
        let mut parsed = ParsedFile {
            path: path.into(),
            hash,
            nodes: vec![file.clone()],
            imports: vec![],
            references: vec![],
            exports: vec![],
            diagnostics: vec![],
            ambiguous: vec![],
            reassigned: vec![],
        };
        let mut stack = vec![(tree.root_node(), file.id, String::new(), String::new())];
        if tree.root_node().has_error() {
            parsed.diagnostics.push(Diagnostic {
                path: path.into(),
                line: 1,
                code: "parse-error".into(),
                message:
                    "Python syntax tree contains errors or missing tokens; analysis is partial"
                        .into(),
            });
        }
        let mut visited = 0;
        while let Some((node, mut owner, mut scope, mut class)) = stack.pop() {
            visited += 1;
            if visited > 300_000 {
                return Err("Python syntax tree limit reached".into());
            }
            if node.is_error() || node.is_missing() {
                parsed.diagnostics.push(Diagnostic {
                    path: path.into(),
                    line: span(node).line,
                    code: "parse-error".into(),
                    message: "Invalid Python syntax; partial analysis only".into(),
                });
            }
            let kind = match node.kind() {
                "function_definition" => Some(if class.is_empty() {
                    "function"
                } else {
                    "method"
                }),
                "class_definition" => Some("class"),
                "assignment" => Some("variable"),
                _ => None,
            };
            if let Some(kind) = kind
                && let Some(name) = field(node, "name")
                    .or_else(|| field(node, "left"))
                    .filter(|name| name.kind() == "identifier")
            {
                let name = text(name, source);
                let qualified = if scope.is_empty() {
                    name.into()
                } else {
                    format!("{scope}.{name}")
                };
                let test =
                    name.starts_with("test_") || (kind == "class" && name.starts_with("Test"));
                let symbol = Node {
                    id: stable_id(root, path, "python", &qualified, kind),
                    name: name.into(),
                    qualified_name: qualified.clone(),
                    kind: kind.into(),
                    path: path.into(),
                    language: "python".into(),
                    span: span(node),
                    exported: scope.is_empty() && !name.starts_with('_'),
                    test,
                    fingerprint: fingerprint(text(node, source)),
                };
                if scope.is_empty() {
                    parsed.exports.push((name.into(), qualified.clone()));
                }
                owner = symbol.id.clone();
                if kind != "variable" {
                    scope = qualified.clone();
                }
                if kind == "class" {
                    class = qualified;
                }
                parsed.nodes.push(symbol);
            }
            if node.kind() == "import_statement" {
                for module in children(node) {
                    let (local, remote) = binding(module, source);
                    let local = if module.kind() == "aliased_import" {
                        local
                    } else {
                        remote.split('.').next().unwrap_or(&remote).into()
                    };
                    parsed.imports.push(Import {
                        specifier: format!("py:{remote}"),
                        bindings: vec![(local, "*".into())],
                        span: span(node),
                        evidence: text(node, source).chars().take(500).collect(),
                        reexport: false,
                        type_only: false,
                    });
                }
                continue;
            }
            if node.kind() == "import_from_statement" {
                let module = field(node, "module_name");
                let specifier = module.map(|module| text(module, source)).unwrap_or("");
                let bindings = children(node)
                    .into_iter()
                    .filter(|child| Some(child.id()) != module.map(|module| module.id()))
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            "dotted_name" | "aliased_import" | "identifier"
                        )
                    })
                    .map(|child| binding(child, source))
                    .collect();
                parsed.imports.push(Import {
                    specifier: format!("py:{specifier}"),
                    bindings,
                    span: span(node),
                    evidence: text(node, source).chars().take(500).collect(),
                    reexport: path.ends_with("__init__.py"),
                    type_only: false,
                });
                continue;
            }
            if node.kind() == "call"
                && let Some(function) = field(node, "function")
            {
                let original = text(function, source);
                let name = if let Some(member) =
                    original.strip_prefix("self.").filter(|_| !class.is_empty())
                {
                    format!("{class}.{member}")
                } else {
                    original.into()
                };
                if matches!(function.kind(), "identifier" | "attribute") {
                    parsed.references.push(Reference {
                        owner: owner.clone(),
                        name,
                        kind: "CALLS".into(),
                        span: span(node),
                        evidence: text(node, source).chars().take(500).collect(),
                        shadowed: !original.starts_with("self.")
                            && shadowed(node, original, source),
                    });
                } else {
                    parsed.diagnostics.push(Diagnostic {
                        path: path.into(),
                        line: span(node).line,
                        code: "dynamic-call".into(),
                        message: "Python call target is selected at runtime".into(),
                    });
                }
            }
            if node.kind() == "identifier"
                && node.parent().is_some_and(|parent| {
                    !matches!(parent.kind(), "parameters" | "attribute")
                        && field(parent, "name").map(|name| name.id()) != Some(node.id())
                        && field(parent, "function").map(|name| name.id()) != Some(node.id())
                })
            {
                parsed.references.push(Reference {
                    owner: owner.clone(),
                    name: text(node, source).into(),
                    kind: "REFERENCES".into(),
                    span: span(node),
                    evidence: text(node, source).into(),
                    shadowed: shadowed(node, text(node, source), source),
                });
            }
            for child in children(node).into_iter().rev() {
                stack.push((child, owner.clone(), scope.clone(), class.clone()));
            }
        }
        let mut ids = std::collections::HashSet::new();
        parsed.nodes.retain(|node| {
            if ids.insert(node.id.clone()) {
                true
            } else {
                parsed.ambiguous.push(node.id.clone());
                parsed.diagnostics.push(Diagnostic {
                    path: path.into(),
                    line: node.span.line,
                    code: "duplicate-symbol".into(),
                    message: format!("Ambiguous Python declaration {}", node.qualified_name),
                });
                false
            }
        });
        Ok(parsed)
    }
}
