use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub path: String,
    pub language: String,
    pub span: Span,
    pub exported: bool,
    pub test: bool,
    #[serde(default)]
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub confidence: String,
    pub path: String,
    pub span: Span,
    pub evidence: String,
    pub adapter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub specifier: String,
    pub bindings: Vec<(String, String)>,
    pub span: Span,
    pub evidence: String,
    pub reexport: bool,
    pub type_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub owner: String,
    pub name: String,
    pub kind: String,
    pub span: Span,
    pub evidence: String,
    pub shadowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub path: String,
    pub hash: String,
    pub nodes: Vec<Node>,
    pub imports: Vec<Import>,
    pub references: Vec<Reference>,
    pub exports: Vec<(String, String)>,
    pub diagnostics: Vec<Diagnostic>,
    pub ambiguous: Vec<String>,
    #[serde(default)]
    pub reassigned: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    pub phase: String,
    pub examined: usize,
    pub source_files: usize,
    pub parsed_files: usize,
    pub reused_files: usize,
    pub symbols: usize,
    pub relationships: usize,
    pub current_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub root: String,
    pub name: String,
    pub indexed_at: u64,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<Diagnostic>,
    pub project_files: Vec<String>,
    pub languages: Vec<String>,
    pub progress: Progress,
}

pub fn stable_id(root: &str, path: &str, language: &str, name: &str, kind: &str) -> String {
    // Length-prefixed fields avoid ambiguous concatenations. Line numbers do not
    // participate: inserting blank lines must not invalidate selection or caches.
    let mut hasher = blake3::Hasher::new();
    for part in [root, path, language, name, kind] {
        hasher.update(&(part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

pub fn fingerprint(source: &str) -> String {
    blake3::hash(source.replace("\r\n", "\n").as_bytes())
        .to_hex()
        .to_string()
}

pub fn edge(
    from: &Node,
    to: &Node,
    kind: &str,
    confidence: &str,
    span: &Span,
    evidence: &str,
) -> Edge {
    let id = stable_id(
        &from.id,
        &to.id,
        kind,
        &format!("{}:{}", span.start_byte, span.end_byte),
        confidence,
    );
    Edge {
        id,
        from: from.id.clone(),
        to: to.id.clone(),
        kind: kind.into(),
        confidence: confidence.into(),
        path: from.path.clone(),
        span: span.clone(),
        evidence: evidence.chars().take(500).collect(),
        adapter: from.language.clone(),
    }
}

pub fn push_edge(
    edges: &mut Vec<Edge>,
    diagnostics: &mut Vec<Diagnostic>,
    progress: &mut Progress,
    edge: Edge,
) {
    const LIMIT: usize = 500_000;
    if edges.len() < LIMIT {
        edges.push(edge);
    } else if progress.relationships != LIMIT + 1 {
        progress.relationships = LIMIT + 1;
        diagnostics.push(Diagnostic {
            path: String::new(),
            line: 0,
            code: "relationship-limit".into(),
            message:
                "Graph exceeds the 500,000 relationship limit; remaining relationships were omitted"
                    .into(),
        });
    }
}
