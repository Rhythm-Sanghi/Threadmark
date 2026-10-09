use threadmark_core::{graph::Graph, model::*};

fn fixture(relations: &[(&str, &str, &str)]) -> Graph {
    let span = Span {
        start_byte: 0,
        end_byte: 1,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 2,
    };
    let nodes: Vec<_> = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|name| Node {
            id: name.to_string(),
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind: "function".into(),
            path: format!("{name}.ts"),
            language: "typescript".into(),
            span: span.clone(),
            exported: true,
            fingerprint: String::new(),
            test: *name == "E",
        })
        .collect();
    let edges = relations
        .iter()
        .map(|(from, to, confidence)| {
            edge(
                nodes.iter().find(|node| node.id == *from).unwrap(),
                nodes.iter().find(|node| node.id == *to).unwrap(),
                "CALLS",
                confidence,
                &span,
                &format!("{to}()"),
            )
        })
        .collect();
    Graph::new(Snapshot {
        schema_version: 1,
        root: "repo".into(),
        name: "repo".into(),
        indexed_at: 0,
        nodes,
        edges,
        diagnostics: Vec::new(),
        project_files: Vec::new(),
        languages: vec!["typescript".into()],
        progress: Progress::default(),
    })
}

#[test]
fn direction_cycles_depth_and_unrelated_branches() {
    let graph = fixture(&[
        ("A", "B", "RESOLVED"),
        ("B", "C", "RESOLVED"),
        ("B", "D", "RESOLVED"),
        ("C", "A", "RESOLVED"),
    ]);
    let impact = graph.impact(&["C".into()], 8, &[]).unwrap();
    assert_eq!(impact.direct[0].node.id, "B");
    assert_eq!(impact.transitive[0].node.id, "A");
    assert!(
        !impact
            .direct
            .iter()
            .chain(&impact.transitive)
            .any(|item| item.node.id == "D")
    );
    assert_eq!(impact.transitive[0].evidence_path.len(), 2);
    let limited = graph.impact(&["C".into()], 1, &[]).unwrap();
    assert!(limited.transitive.is_empty());
    assert!(limited.truncated);
}

#[test]
fn exact_paths_dominate_inferred_paths_and_tests_keep_evidence() {
    let graph = fixture(&[
        ("B", "C", "INFERRED"),
        ("A", "B", "RESOLVED"),
        ("A", "C", "RESOLVED"),
        ("E", "A", "RESOLVED"),
    ]);
    let impact = graph.impact(&["C".into()], 8, &[]).unwrap();
    assert_eq!(impact.possible.len(), 1);
    assert_eq!(impact.possible[0].node.id, "B");
    assert_eq!(impact.direct[0].node.id, "A");
    assert_eq!(impact.related_tests[0].node.id, "E");
    assert_eq!(impact.related_tests[0].evidence_path.len(), 2);
}

#[test]
fn diamond_paths_use_shortest_evidence_and_large_results_are_marked() {
    let graph = fixture(&[
        ("A", "B", "RESOLVED"),
        ("A", "C", "RESOLVED"),
        ("B", "D", "RESOLVED"),
        ("C", "D", "RESOLVED"),
    ]);
    let impact = graph.impact(&["D".into()], 8, &[]).unwrap();
    assert_eq!(impact.direct.len(), 2);
    let caller = impact
        .transitive
        .iter()
        .find(|item| item.node.id == "A")
        .unwrap();
    assert_eq!(caller.hops, 2);
    assert_eq!(caller.evidence_path.len(), 2);
    assert_eq!(caller.evidence_path[0].from, "A");
    assert_eq!(caller.evidence_path[1].to, "D");
    let mut snapshot = graph.snapshot;
    let target = snapshot.nodes[3].clone();
    let template = snapshot.nodes[0].clone();
    snapshot.nodes = vec![target.clone()];
    snapshot.edges.clear();
    for index in 0..2100 {
        let mut node = template.clone();
        node.id = format!("caller{index}");
        node.name = node.id.clone();
        node.qualified_name = node.id.clone();
        snapshot
            .edges
            .push(edge(&node, &target, "CALLS", "RESOLVED", &node.span, "D()"));
        snapshot.nodes.push(node);
    }
    let impact = Graph::new(snapshot).impact(&["D".into()], 8, &[]).unwrap();
    assert!(impact.truncated);
    assert_eq!(impact.direct.len(), 2000);
}
