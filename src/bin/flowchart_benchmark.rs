use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use selkie::editable::{flowchart_db_from_graph, graph_to_mermaid_text, parse_to_graph};
use selkie::layout::{self, CharacterSizeEstimator, ToLayoutGraph};
use selkie::render::svg::{RenderConfig, SvgRenderer};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Serialize)]
struct BenchmarkCase {
    input: String,
    node_count: usize,
    edge_count: usize,
    parse_ms: f64,
    layout_ms: f64,
    render_ms: f64,
    serialize_ms: f64,
    total_ms: f64,
    svg_bytes: usize,
    mermaid_bytes: usize,
    svg_contains_all_node_labels: bool,
    edge_marker_count: usize,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (output, inputs) = parse_args(args)?;
    if inputs.is_empty() {
        return Err("usage: flowchart_benchmark [--output report.json] <files...>".into());
    }

    let mut cases = Vec::new();
    for input in inputs {
        cases.push(benchmark_file(input)?);
    }

    let report = BenchmarkReport { cases };
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(output) = output {
        fs::write(output, format!("{json}\n"))?;
    } else {
        println!("{json}");
    }
    Ok(())
}

fn parse_args(
    args: Vec<String>,
) -> Result<(Option<PathBuf>, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let mut output = None;
    let mut inputs = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "--output" || arg == "-o" {
            let Some(path) = iter.next() else {
                return Err("--output requires a path".into());
            };
            output = Some(PathBuf::from(path));
        } else {
            inputs.push(PathBuf::from(arg));
        }
    }
    Ok((output, inputs))
}

fn benchmark_file(path: PathBuf) -> Result<BenchmarkCase, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(&path)?;
    let total_start = Instant::now();

    let parse_start = Instant::now();
    let graph = parse_to_graph(&source)?;
    let parse_ms = elapsed_ms(parse_start);

    let serialize_start = Instant::now();
    let mermaid_text = graph_to_mermaid_text(&graph)?;
    let serialize_ms = elapsed_ms(serialize_start);

    let db = flowchart_db_from_graph(&graph)?;
    let size_estimator = CharacterSizeEstimator::default();

    let layout_start = Instant::now();
    let layout_graph = db.to_layout_graph(&size_estimator)?;
    let layout_graph = layout::layout(layout_graph)?;
    let layout_ms = elapsed_ms(layout_start);

    let render_start = Instant::now();
    let renderer = SvgRenderer::new(RenderConfig::default());
    let svg = renderer.render_flowchart(&db, &layout_graph)?;
    let render_ms = elapsed_ms(render_start);

    let total_ms = elapsed_ms(total_start);
    let svg_contains_all_node_labels = graph
        .nodes
        .iter()
        .all(|node| svg.contains(&format!(">{}<", node.label)));
    let edge_marker_count = svg.matches("class=\"edge").count();

    if !svg.starts_with("<svg") && !svg.contains("<svg") {
        return Err(format!("{} did not render valid SVG", path.display()).into());
    }
    if !svg_contains_all_node_labels {
        return Err(format!("{} SVG is missing at least one node label", path.display()).into());
    }
    if edge_marker_count < graph.edges.len() {
        return Err(format!(
            "{} SVG edge marker count {} is below edge count {}",
            path.display(),
            edge_marker_count,
            graph.edges.len()
        )
        .into());
    }

    Ok(BenchmarkCase {
        input: path.display().to_string(),
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        parse_ms,
        layout_ms,
        render_ms,
        serialize_ms,
        total_ms,
        svg_bytes: svg.len(),
        mermaid_bytes: mermaid_text.len(),
        svg_contains_all_node_labels,
        edge_marker_count,
    })
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
