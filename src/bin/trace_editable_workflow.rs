use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use selkie::editable::{
    apply_graph_patch_result_json, graph_to_mermaid_text, layout_editable_graph, parse_to_graph,
    render_graph_parts_with_layout_mode, EditableDiagram, EditableLayoutMode,
};
use serde::Serialize;
use serde_json::json;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Serialize)]
struct WorkflowSummary {
    input: String,
    nodes_before: usize,
    edges_before: usize,
    nodes_after: usize,
    edges_after: usize,
    render_part_nodes: usize,
    render_part_edges: usize,
    render_part_labels: usize,
    exported_bytes: usize,
    reimported_nodes: usize,
    reimported_edges: usize,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing()?;

    let args = env::args().skip(1).collect::<Vec<_>>();
    let (input, summary_output) = parse_args(args)?;
    let source = fs::read_to_string(&input)?;

    let graph = {
        let span = tracing::trace_span!(
            target: "selkie::phase6",
            "phase6.editable.parse_to_graph",
            input_bytes = source.len() as u64
        );
        let _enter = span.enter();
        parse_to_graph(&source)?
    };
    let nodes_before = graph.nodes.len();
    let edges_before = graph.edges.len();

    let graph = {
        let span = tracing::trace_span!(
            target: "selkie::phase6",
            "phase6.editable.full_layout",
            nodes = graph.nodes.len() as u64,
            edges = graph.edges.len() as u64
        );
        let _enter = span.enter();
        layout_editable_graph(&graph)?
    };

    let parts = {
        let span = tracing::trace_span!(
            target: "selkie::phase6",
            "phase6.editable.render_graph_parts",
            nodes = graph.nodes.len() as u64,
            edges = graph.edges.len() as u64
        );
        let _enter = span.enter();
        render_graph_parts_with_layout_mode(&graph, EditableLayoutMode::Edit)?
    };

    let first_node_id = graph
        .nodes
        .first()
        .ok_or("editable workflow fixture must contain at least one node")?
        .id
        .clone();
    let mut graph = move_node(graph, &first_node_id)?;
    graph = create_node(graph)?;
    graph = create_edge(graph, &first_node_id)?;

    let exported = {
        let span = tracing::trace_span!(
            target: "selkie::phase6",
            "phase6.editable.export",
            nodes = graph.nodes.len() as u64,
            edges = graph.edges.len() as u64,
            output_bytes = tracing::field::Empty
        );
        let _enter = span.enter();
        let text = graph_to_mermaid_text(&graph)?;
        span.record("output_bytes", text.len() as u64);
        text
    };

    let reimported = {
        let span = tracing::trace_span!(
            target: "selkie::phase6",
            "phase6.editable.re_import",
            input_bytes = exported.len() as u64
        );
        let _enter = span.enter();
        parse_to_graph(&exported)?
    };

    let summary = WorkflowSummary {
        input: input.display().to_string(),
        nodes_before,
        edges_before,
        nodes_after: graph.nodes.len(),
        edges_after: graph.edges.len(),
        render_part_nodes: parts.nodes.len(),
        render_part_edges: parts.edges.len(),
        render_part_labels: parts.labels.len(),
        exported_bytes: exported.len(),
        reimported_nodes: reimported.nodes.len(),
        reimported_edges: reimported.edges.len(),
    };
    write_summary(summary_output, &summary)?;
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, Option<PathBuf>), Box<dyn std::error::Error>> {
    let mut input = None;
    let mut summary_output = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "--summary-output" {
            let Some(path) = iter.next() else {
                return Err("--summary-output requires a path".into());
            };
            summary_output = Some(PathBuf::from(path));
        } else if input.is_none() {
            input = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
    }

    let Some(input) = input else {
        return Err(
            "usage: trace-editable-workflow <input.mmd> [--summary-output summary.json]".into(),
        );
    };
    Ok((input, summary_output))
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let filter = env::var("SELKIE_TRACE_FILTER").unwrap_or_else(|_| "selkie=trace".to_string());
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_new(filter)?)
        .with_span_events(FmtSpan::CLOSE)
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(io::stderr)
        .try_init()
        .map_err(|err| io::Error::other(err.to_string()))?;
    Ok(())
}

fn move_node(
    graph: EditableDiagram,
    node_id: &str,
) -> Result<EditableDiagram, Box<dyn std::error::Error>> {
    let graph_json = serde_json::to_string(&graph)?;
    let patch = json!({
        "op": "move_node",
        "id": node_id,
        "x": 240.0,
        "y": 160.0,
        "locked": true
    });
    let span = tracing::trace_span!(
        target: "selkie::phase6",
        "phase6.editable.move_node",
        node_id = node_id,
        nodes = graph.nodes.len() as u64,
        edges = graph.edges.len() as u64
    );
    let _enter = span.enter();
    apply_patch(graph_json, patch)
}

fn create_node(graph: EditableDiagram) -> Result<EditableDiagram, Box<dyn std::error::Error>> {
    let graph_json = serde_json::to_string(&graph)?;
    let patch = json!({
        "op": "add_node",
        "node": {
            "id": "TraceNode",
            "label": "Trace Node",
            "shape": "square",
            "classes": [],
            "styles": ["fill:#f8fafc", "stroke:#0f766e"],
            "position": {
                "x": 360.0,
                "y": 180.0,
                "locked": true
            }
        }
    });
    let span = tracing::trace_span!(
        target: "selkie::phase6",
        "phase6.editable.create_node",
        nodes = graph.nodes.len() as u64,
        edges = graph.edges.len() as u64
    );
    let _enter = span.enter();
    apply_patch(graph_json, patch)
}

fn create_edge(
    graph: EditableDiagram,
    source_id: &str,
) -> Result<EditableDiagram, Box<dyn std::error::Error>> {
    let graph_json = serde_json::to_string(&graph)?;
    let patch = json!({
        "op": "add_edge",
        "edge": {
            "id": "trace_edge",
            "source": source_id,
            "target": "TraceNode",
            "label": "trace",
            "edge_type": "-->",
            "stroke": "normal",
            "classes": [],
            "styles": ["stroke:#0f766e"]
        }
    });
    let span = tracing::trace_span!(
        target: "selkie::phase6",
        "phase6.editable.create_edge",
        source_id = source_id,
        nodes = graph.nodes.len() as u64,
        edges = graph.edges.len() as u64
    );
    let _enter = span.enter();
    apply_patch(graph_json, patch)
}

fn apply_patch(
    graph_json: String,
    patch: serde_json::Value,
) -> Result<EditableDiagram, Box<dyn std::error::Error>> {
    let result_json = apply_graph_patch_result_json(&graph_json, &patch.to_string())?;
    let result: selkie::editable::EditablePatchResult = serde_json::from_str(&result_json)?;
    Ok(result.graph)
}

fn write_summary(
    summary_output: Option<PathBuf>,
    summary: &WorkflowSummary,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(summary)?;
    if let Some(path) = summary_output {
        fs::write(path, format!("{json}\n"))?;
    } else {
        println!("{json}");
    }
    Ok(())
}
