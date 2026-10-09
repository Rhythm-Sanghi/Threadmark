use crate::model::*;
use tree_sitter::{Node as Syntax, Parser};

pub const ADAPTER_VERSION: &str = "js-ts-8-python-3";

pub trait LanguageAdapter {
    fn parse(&self, root: &str, path: &str, source: &str) -> Result<ParsedFile, String>;
}

pub struct JavaScript;

pub fn language(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()? {
        "ts" | "tsx" | "mts" | "cts" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" => Some("python"),
        _ => None,
    }
}

pub fn span(node: Syntax<'_>) -> Span {
    Span {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        line: node.start_position().row + 1,
        column: node.start_position().column + 1,
        end_line: node.end_position().row + 1,
        end_column: node.end_position().column + 1,
    }
}

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
fn string(node: Syntax<'_>, source: &str) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let value = text(node, source);
    // Escape decoding is deliberately rejected rather than resolving the wrong file.
    if value.contains('\\') {
        return None;
    }
    Some(value[1..value.len().saturating_sub(1)].into())
}

fn evidence(node: Syntax<'_>, source: &str) -> String {
    text(node, source).chars().take(500).collect()
}

fn parameter_shadows(mut node: Syntax<'_>, name: &str, source: &str) -> bool {
    let name = name.split('.').next().unwrap_or(name);
    while let Some(parent) = node.parent() {
        if let Some(parameters) = field(parent, "parameters").or_else(|| field(parent, "parameter"))
        {
            let mut stack = vec![parameters];
            while let Some(parameter) = stack.pop() {
                if parameter.kind() == "identifier" && text(parameter, source) == name {
                    return true;
                }
                if !matches!(parameter.kind(), "type_annotation" | "type_identifier") {
                    stack.extend(children(parameter));
                }
            }
        }
        node = parent;
    }
    false
}

impl LanguageAdapter for JavaScript {
    fn parse(&self, root: &str, path: &str, source: &str) -> Result<ParsedFile, String> {
        let language = language(path).ok_or("Unsupported language")?;
        if language == "python" {
            return crate::python::Python.parse(root, path, source);
        }
        let grammar = if path.ends_with(".tsx") {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else if language == "typescript" {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        } else {
            tree_sitter_javascript::LANGUAGE.into()
        };
        let mut parser = Parser::new();
        parser
            .set_language(&grammar)
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
            .ok_or("Parser exceeded its two second time budget")?;
        let file = Node {
            id: stable_id(root, path, language, path, "file"),
            name: path.rsplit('/').next().unwrap_or(path).into(),
            qualified_name: path.into(),
            kind: "file".into(),
            path: path.into(),
            language: language.into(),
            span: span(tree.root_node()),
            fingerprint: fingerprint(source),
            exported: false,
            test: path.contains(".test.")
                || path.contains(".spec.")
                || path.starts_with("tests/")
                || path.contains("/__tests__/"),
        };
        let mut parsed = ParsedFile {
            path: path.into(),
            hash: blake3::hash(source.as_bytes()).to_hex().to_string(),
            nodes: vec![file.clone()],
            imports: Vec::new(),
            references: Vec::new(),
            exports: Vec::new(),
            diagnostics: Vec::new(),
            ambiguous: Vec::new(),
            reassigned: Vec::new(),
        };
        let mut stack = vec![(tree.root_node(), file.id, String::new(), false)];
        if tree.root_node().has_error() {
            parsed.diagnostics.push(Diagnostic {
                path: path.into(),
                line: 1,
                code: "parse-error".into(),
                message: "Syntax tree contains errors or missing tokens; analysis is partial"
                    .into(),
            });
        }
        let mut visited = 0;
        let mut routers = std::collections::HashSet::new();
        let mut mutations = Vec::new();
        while let Some((node, mut owner, mut scope, mut exported)) = stack.pop() {
            visited += 1;
            if visited > 300_000 {
                return Err("Syntax tree exceeds the 300,000 node limit".into());
            }
            if node.is_error() || node.is_missing() {
                parsed.diagnostics.push(Diagnostic {
                    path: path.into(),
                    line: span(node).line,
                    code: "parse-error".into(),
                    message: format!(
                        "Invalid syntax near {}:{}; partial analysis only",
                        span(node).line,
                        span(node).column
                    ),
                });
            }
            if node.kind() == "export_statement" {
                exported = true;
                if let Some(source_node) = field(node, "source") {
                    if let Some(specifier) = string(source_node, source) {
                        let mut bindings = Vec::new();
                        for clause in children(node) {
                            for item in children(clause) {
                                if item.kind() == "export_specifier" {
                                    let name =
                                        field(item, "name").map(|n| text(n, source)).unwrap_or("");
                                    let alias = field(item, "alias")
                                        .map(|n| text(n, source))
                                        .unwrap_or(name);
                                    bindings.push((alias.into(), name.into()));
                                }
                            }
                        }
                        parsed.imports.push(Import {
                            specifier,
                            bindings,
                            span: span(node),
                            evidence: evidence(node, source),
                            reexport: true,
                            type_only: text(node, source).starts_with("export type"),
                        });
                    }
                } else {
                    for clause in children(node) {
                        for item in children(clause) {
                            if item.kind() == "export_specifier" {
                                let name =
                                    field(item, "name").map(|n| text(n, source)).unwrap_or("");
                                let alias = field(item, "alias")
                                    .map(|n| text(n, source))
                                    .unwrap_or(name);
                                parsed.exports.push((alias.into(), name.into()));
                            }
                        }
                    }
                }
            }
            let symbol_kind = match node.kind() {
                "function_declaration"
                | "generator_function_declaration"
                | "function_signature" => Some("function"),
                "function_expression" | "arrow_function"
                    if node
                        .parent()
                        .is_some_and(|p| p.kind() == "export_statement") =>
                {
                    Some("function")
                }
                "class_declaration" | "abstract_class_declaration" => Some("class"),
                "method_definition" | "method_signature" => Some("method"),
                "interface_declaration" => Some("interface"),
                "type_alias_declaration" => Some("type"),
                "enum_declaration" => Some("enum"),
                "variable_declarator" => Some(
                    if field(node, "value").is_some_and(|v| {
                        matches!(
                            v.kind(),
                            "arrow_function" | "function_expression" | "generator_function"
                        )
                    }) {
                        "function"
                    } else {
                        "variable"
                    },
                ),
                _ => None,
            };
            if let Some(kind) = symbol_kind {
                let name_node = field(node, "name").filter(|n| {
                    matches!(
                        n.kind(),
                        "identifier"
                            | "type_identifier"
                            | "property_identifier"
                            | "private_property_identifier"
                    )
                });
                let default_export = node.parent().is_some_and(|p| {
                    p.kind() == "export_statement" && text(p, source).starts_with("export default")
                });
                if name_node.is_some() || default_export {
                    let name = name_node
                        .map(|n| text(n, source))
                        .unwrap_or("default")
                        .to_string();
                    let qualified = if scope.is_empty() {
                        name.clone()
                    } else {
                        format!("{scope}.{name}")
                    };
                    let symbol = Node {
                        id: stable_id(root, path, language, &qualified, kind),
                        name: name.clone(),
                        qualified_name: qualified.clone(),
                        kind: kind.into(),
                        path: path.into(),
                        language: language.into(),
                        span: span(node),
                        exported,
                        test: false,
                        fingerprint: fingerprint(text(node, source)),
                    };
                    if exported {
                        parsed.exports.push((name.clone(), qualified.clone()));
                        let parent = node.parent().map(|n| text(n, source)).unwrap_or("");
                        if parent.starts_with("export default") {
                            parsed.exports.push(("default".into(), qualified.clone()));
                        }
                    }
                    owner = symbol.id.clone();
                    if matches!(kind, "function" | "class" | "method" | "interface") {
                        scope = qualified;
                        exported = false;
                    }
                    parsed.nodes.push(symbol);
                }
            }
            if node.kind() == "import_statement"
                && let Some(specifier) = field(node, "source").and_then(|n| string(n, source))
            {
                let mut bindings = Vec::new();
                for clause in children(node) {
                    if clause.kind() == "import_clause" {
                        for item in children(clause) {
                            match item.kind() {
                                "identifier" => {
                                    bindings.push((text(item, source).into(), "default".into()))
                                }
                                "namespace_import" => {
                                    if let Some(local) = children(item).last() {
                                        bindings.push((text(*local, source).into(), "*".into()));
                                    }
                                }
                                "named_imports" => {
                                    for spec in children(item) {
                                        let name = field(spec, "name")
                                            .map(|n| text(n, source))
                                            .unwrap_or("");
                                        let alias = field(spec, "alias")
                                            .map(|n| text(n, source))
                                            .unwrap_or(name);
                                        bindings.push((alias.into(), name.into()));
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                }
                parsed.imports.push(Import {
                    specifier,
                    bindings,
                    span: span(node),
                    evidence: evidence(node, source),
                    reexport: false,
                    type_only: text(node, source).starts_with("import type"),
                });
            }
            if node.kind() == "call_expression"
                && let Some(function) = field(node, "function")
            {
                let callee = text(function, source);
                let args = field(node, "arguments").map(children).unwrap_or_default();
                if let Some(declaration) = node
                    .parent()
                    .filter(|parent| parent.kind() == "variable_declarator")
                    && declaration
                        .parent()
                        .is_some_and(|parent| text(parent, source).starts_with("const "))
                    && let Some(name) =
                        field(declaration, "name").filter(|name| name.kind() == "identifier")
                    && parsed.imports.iter().any(|import| {
                        import.specifier == "express"
                            && import.bindings.iter().any(|(local, remote)| {
                                (remote == "default"
                                    && (callee == local || callee == format!("{local}.Router")))
                                    || (remote == "Router" && callee == local)
                            })
                    })
                {
                    routers.insert(format!("{scope}|{}", text(name, source)));
                }
                if let Some((receiver, method)) = callee.rsplit_once('.')
                    && routers.contains(&format!("{scope}|{receiver}"))
                    && !parameter_shadows(node, receiver, source)
                    && matches!(
                        method,
                        "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "all"
                    )
                    && let Some(route_path) =
                        args.first().and_then(|argument| string(*argument, source))
                {
                    let name = format!("{} {}", method.to_uppercase(), route_path);
                    let qualified = format!("route:{receiver}:{name}");
                    let route = Node {
                        id: stable_id(root, path, language, &qualified, "route"),
                        name,
                        qualified_name: qualified.clone(),
                        kind: "route".into(),
                        path: path.into(),
                        language: language.into(),
                        span: span(node),
                        exported: false,
                        test: false,
                        fingerprint: fingerprint(text(node, source)),
                    };
                    for handler in args.iter().skip(1).filter(|handler| {
                        matches!(handler.kind(), "identifier" | "member_expression")
                    }) {
                        parsed.references.push(Reference {
                            owner: route.id.clone(),
                            name: text(*handler, source).into(),
                            kind: "ROUTES_TO".into(),
                            span: span(node),
                            evidence: evidence(node, source),
                            shadowed: parameter_shadows(node, text(*handler, source), source),
                        });
                    }
                    owner = route.id.clone();
                    scope = qualified;
                    parsed.nodes.push(route);
                }
                let base = callee.split('.').next().unwrap_or(callee);
                let known_test = parsed.imports.iter().any(|import| {
                    matches!(
                        import.specifier.as_str(),
                        "vitest" | "@jest/globals" | "node:test"
                    ) && import.bindings.iter().any(|(local, remote)| {
                        local == base
                            && matches!(remote.as_str(), "test" | "it" | "describe" | "default")
                    })
                });
                if (known_test
                    || (parsed.nodes[0].test && matches!(base, "test" | "it" | "describe")))
                    && args.get(1).is_some_and(|argument| {
                        matches!(argument.kind(), "arrow_function" | "function_expression")
                    })
                    && let Some(description) =
                        args.first().and_then(|argument| string(*argument, source))
                {
                    let qualified = if scope.is_empty() {
                        format!("test:{description}")
                    } else {
                        format!("{scope}.test:{description}")
                    };
                    let test = Node {
                        id: stable_id(root, path, language, &qualified, "test"),
                        name: description,
                        qualified_name: qualified.clone(),
                        kind: "test".into(),
                        path: path.into(),
                        language: language.into(),
                        span: span(node),
                        exported: false,
                        test: true,
                        fingerprint: fingerprint(text(node, source)),
                    };
                    owner = test.id.clone();
                    scope = qualified;
                    parsed.nodes.push(test);
                }
                if callee == "import" || callee == "require" {
                    if let Some(specifier) = args.first().and_then(|n| string(*n, source)) {
                        let bindings = if callee == "require" {
                            node.parent()
                                .filter(|p| p.kind() == "variable_declarator")
                                .and_then(|p| field(p, "name"))
                                .filter(|n| n.kind() == "identifier")
                                .map(|n| vec![(text(n, source).into(), "*".into())])
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        parsed.imports.push(Import {
                            specifier,
                            bindings,
                            span: span(node),
                            evidence: evidence(node, source),
                            reexport: false,
                            type_only: false,
                        });
                    } else {
                        parsed.diagnostics.push(Diagnostic {
                            path: path.into(),
                            line: span(node).line,
                            code: "dynamic-import".into(),
                            message: "Import target is selected at runtime".into(),
                        });
                    }
                } else if function.kind() == "identifier" || function.kind() == "member_expression"
                {
                    parsed.references.push(Reference {
                        owner: owner.clone(),
                        name: callee.into(),
                        kind: "CALLS".into(),
                        span: span(node),
                        evidence: evidence(node, source),
                        shadowed: parameter_shadows(node, callee, source),
                    });
                } else {
                    parsed.diagnostics.push(Diagnostic {
                        path: path.into(),
                        line: span(node).line,
                        code: "dynamic-call".into(),
                        message: format!(
                            "Call target is selected dynamically: {}",
                            callee.chars().take(120).collect::<String>()
                        ),
                    });
                }
            }
            if node.kind() == "type_identifier"
                && node
                    .parent()
                    .is_some_and(|p| field(p, "name").map(|n| n.id()) != Some(node.id()))
            {
                parsed.references.push(Reference {
                    owner: owner.clone(),
                    name: text(node, source).into(),
                    kind: "USES_TYPE".into(),
                    span: span(node),
                    evidence: evidence(node, source),
                    shadowed: false,
                });
            }
            if matches!(
                node.kind(),
                "jsx_opening_element" | "jsx_self_closing_element"
            ) && let Some(name) = field(node, "name")
                && text(name, source)
                    .chars()
                    .next()
                    .is_some_and(char::is_uppercase)
            {
                parsed.references.push(Reference {
                    owner: owner.clone(),
                    name: text(name, source).into(),
                    kind: "REFERENCES".into(),
                    span: span(node),
                    evidence: evidence(node, source),
                    shadowed: parameter_shadows(node, text(name, source), source),
                });
            }
            if node.kind() == "member_expression"
                && let Some(property) = field(node, "property")
                && field(node, "object").is_some_and(|object| text(object, source) == "process.env")
                && !parameter_shadows(node, "process", source)
            {
                let name = text(property, source);
                let qualified = format!("config:{name}");
                let configuration = Node {
                    id: stable_id(root, "", "configuration", name, "configuration_key"),
                    name: name.into(),
                    qualified_name: qualified.clone(),
                    kind: "configuration_key".into(),
                    path: path.into(),
                    language: "configuration".into(),
                    span: span(node),
                    exported: false,
                    test: false,
                    fingerprint: blake3::hash(name.as_bytes()).to_hex().to_string(),
                };
                if !parsed
                    .nodes
                    .iter()
                    .any(|existing| existing.id == configuration.id)
                {
                    parsed.nodes.push(configuration);
                }
                parsed.references.push(Reference {
                    owner: owner.clone(),
                    name: qualified,
                    kind: "READS_CONFIG".into(),
                    span: span(node),
                    evidence: text(node, source).into(),
                    shadowed: false,
                });
            }
            if node.kind() == "assignment_expression"
                && let (Some(left), Some(right)) = (field(node, "left"), field(node, "right"))
            {
                let left = text(left, source);
                if left == "module.exports" && right.kind() == "object" {
                    for property in children(right) {
                        if property.kind() == "shorthand_property_identifier" {
                            let name = text(property, source);
                            parsed.exports.push((name.into(), name.into()));
                        } else if let (Some(key), Some(value)) =
                            (field(property, "key"), field(property, "value"))
                            && value.kind() == "identifier"
                        {
                            parsed
                                .exports
                                .push((text(key, source).into(), text(value, source).into()));
                        }
                    }
                } else if right.kind() == "identifier" {
                    if let Some(name) = left
                        .strip_prefix("exports.")
                        .or_else(|| left.strip_prefix("module.exports."))
                    {
                        parsed
                            .exports
                            .push((name.into(), text(right, source).into()));
                    } else if left == "module.exports" {
                        parsed
                            .exports
                            .push(("default".into(), text(right, source).into()));
                    }
                }
            }
            if matches!(
                node.kind(),
                "assignment_expression" | "augmented_assignment_expression" | "update_expression"
            ) && let Some(binding) = field(node, "left").or_else(|| field(node, "argument"))
                && binding.kind() == "identifier"
            {
                mutations.push((scope.clone(), text(binding, source).to_owned(), span(node)));
            }
            let child_nodes = children(node);
            for child in child_nodes.into_iter().rev() {
                stack.push((child, owner.clone(), scope.clone(), exported));
            }
        }
        // Overloads and duplicate declarations are ambiguous, never silently merged.
        let mut identities = std::collections::HashSet::new();
        parsed.nodes.retain(|node| {
            if identities.insert(node.id.clone()) {
                true
            } else {
                parsed.ambiguous.push(node.id.clone());
                parsed.diagnostics.push(Diagnostic {
                    path: path.into(),
                    line: node.span.line,
                    code: "duplicate-symbol".into(),
                    message: format!(
                        "Multiple declarations of {}; only the first declaration is navigable",
                        node.qualified_name
                    ),
                });
                false
            }
        });
        let declarations: std::collections::HashSet<_> = parsed
            .nodes
            .iter()
            .filter(|node| node.kind != "file")
            .map(|node| node.qualified_name.as_str())
            .collect();
        let mut changed = std::collections::BTreeSet::new();
        for (mut scope, name, span) in mutations {
            let qualified = loop {
                let candidate = if scope.is_empty() {
                    name.clone()
                } else {
                    format!("{scope}.{name}")
                };
                if declarations.contains(candidate.as_str()) || scope.is_empty() {
                    break candidate;
                }
                scope = scope
                    .rsplit_once('.')
                    .map(|(parent, _)| parent.to_owned())
                    .unwrap_or_default();
            };
            if changed.insert(qualified.clone()) {
                parsed.diagnostics.push(Diagnostic {path:path.into(),line:span.line,code:"reassigned-binding".into(),message:format!("{qualified} is reassigned; calls through this binding are not resolved to its original declaration")});
            }
        }
        parsed.reassigned = changed.into_iter().collect();
        Ok(parsed)
    }
}
