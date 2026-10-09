use std::sync::atomic::AtomicBool;
use threadmark_core::{Engine, graph::Graph};
#[test]
fn express_routes_and_jsx_have_static_evidence_without_guessing_receivers() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("handler.ts"),
        "export function forecast(){} export function Card(){return null;}",
    )
    .unwrap();
    std::fs::write(root.path().join("routes.ts"),"import express from 'express'; import {forecast} from './handler'; const router=express.Router(); router.post('/forecast',forecast); unknown.get('/invented',forecast);").unwrap();
    std::fs::write(
        root.path().join("View.tsx"),
        "import {Card} from './handler'; export function View(){return <Card/>;}",
    )
    .unwrap();
    let snapshot = Engine::open(Some(data.path()))
        .unwrap()
        .index_repository(root.path(), &AtomicBool::new(false), |_| {})
        .unwrap();
    let graph = Graph::new(snapshot);
    assert_eq!(
        graph
            .snapshot
            .nodes
            .iter()
            .filter(|node| node.kind == "route")
            .count(),
        1
    );
    let route = graph.select("POST /forecast").unwrap();
    let handler = graph.select("forecast").unwrap();
    assert!(graph.snapshot.edges.iter().any(|edge| edge.from == route.id
        && edge.to == handler.id
        && edge.kind == "ROUTES_TO"
        && edge.evidence.contains("router.post")));
    let view = graph.select("View").unwrap();
    let card = graph.select("Card").unwrap();
    assert!(graph.snapshot.edges.iter().any(|edge| edge.from == view.id
        && edge.to == card.id
        && edge.kind == "REFERENCES"
        && edge.evidence == "<Card/>"));
    assert!(
        graph
            .impact(std::slice::from_ref(&handler.id), 8, &[])
            .unwrap()
            .direct
            .iter()
            .any(|item| item.node.id == route.id)
    );
}
