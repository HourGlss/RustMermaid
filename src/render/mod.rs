//! Rendering engine for mermaid diagrams
//!
//! This module provides SVG rendering for positioned diagram elements.

mod architecture;
pub mod ascii;
mod block;
mod c4;
mod class;
mod er;
mod flowchart;
mod gantt;
mod git;
mod journey;
mod kanban;
mod mindmap;
mod packet;
mod pie;
mod quadrant;
mod radar;
mod requirement;
mod sankey;
mod sequence;
mod state;
pub mod svg;
pub(crate) mod text_utils;
mod timeline;
mod treemap;
mod xychart;

use crate::diagrams::{detect_init, detect_type, parse, remove_directives, Diagram};
use crate::error::{MermaidError, Result};
use crate::layout::{self, CharacterSizeEstimator, ToLayoutGraph};

pub use svg::{RenderConfig, SvgRenderer, Theme};

/// Render a diagram to SVG
pub fn render(diagram: &Diagram) -> Result<String> {
    render_with_config(diagram, &RenderConfig::default())
}

/// Render diagram text to SVG with automatic directive processing
///
/// This function:
/// 1. Detects and parses `%%{init: ...}%%` directives
/// 2. Extracts theme configuration from directives
/// 3. Detects the diagram type
/// 4. Parses the diagram
/// 5. Renders with directive-derived theme configuration
///
/// # Example
///
/// ```
/// use selkie::render::render_text;
///
/// let svg = render_text(r#"%%{init: {"theme": "dark"}}%%
/// flowchart TD
///     A[Start] --> B[End]
/// "#).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn render_text(text: &str) -> Result<String> {
    let span = tracing::trace_span!(
        "selkie.render_text",
        input_bytes = text.len() as u64,
        diagram_type = tracing::field::Empty,
        output_bytes = tracing::field::Empty,
    );
    let _enter = span.enter();

    // Extract directive configuration
    let directive_config = detect_init(text);

    // Build render config with directive theme and themeCSS
    let config = if let Some(ref dc) = directive_config {
        RenderConfig {
            theme: Theme::from_directive(dc),
            theme_css: dc.theme_css.clone(),
            ..RenderConfig::default()
        }
    } else {
        RenderConfig::default()
    };

    // Remove directives from text before parsing
    let clean_text = remove_directives(text);

    // Detect diagram type and parse
    let diagram_type = detect_type(&clean_text)?;
    span.record("diagram_type", tracing::field::debug(&diagram_type));
    let diagram = parse(diagram_type, &clean_text)?;

    // Render with config
    let svg = render_with_config(&diagram, &config)?;
    span.record("output_bytes", svg.len() as u64);
    Ok(svg)
}

/// Render a diagram to SVG with custom configuration
pub fn render_with_config(diagram: &Diagram, config: &RenderConfig) -> Result<String> {
    let span = tracing::trace_span!(
        "selkie.render_with_config",
        diagram_type = diagram_type_name(diagram),
        output_bytes = tracing::field::Empty,
    );
    let _enter = span.enter();

    let svg = render_primary_diagram(diagram, config)
        .or_else(|| render_secondary_diagram(diagram, config))
        .or_else(|| render_tertiary_diagram(diagram, config))
        .unwrap_or_else(|| {
            Err(MermaidError::RenderError(format!(
                "Diagram type {:?} not yet supported for rendering",
                diagram_type_name(diagram)
            )))
        })?;
    span.record("output_bytes", svg.len() as u64);
    Ok(svg)
}

fn render_primary_diagram(diagram: &Diagram, config: &RenderConfig) -> Option<Result<String>> {
    match diagram {
        Diagram::Architecture(db) => Some(render_architecture(db, config)),
        Diagram::Block(db) => Some(block::render_block(db, config)),
        Diagram::C4(db) => Some(c4::render_c4(db, config)),
        Diagram::Class(db) => Some(class::render_class(db, config)),
        Diagram::Er(db) => Some(er::render_er(db, config)),
        Diagram::Flowchart(db) => Some(render_flowchart(db, config)),
        Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            Some(gantt::render_gantt(&mut db_clone, config))
        }
        Diagram::Git(db) => Some(git::render_git(db, config)),
        _ => None,
    }
}

fn render_secondary_diagram(diagram: &Diagram, config: &RenderConfig) -> Option<Result<String>> {
    match diagram {
        Diagram::Journey(db) => Some(journey::render_journey(db, config)),
        Diagram::Kanban(db) => Some(kanban::render_kanban(db, config)),
        Diagram::Mindmap(db) => Some(mindmap::render_mindmap(db, config)),
        Diagram::Packet(db) => Some(packet::render_packet(db, config)),
        Diagram::Pie(db) => Some(pie::render_pie(db, config)),
        Diagram::Quadrant(db) => Some(quadrant::render_quadrant(db, config)),
        Diagram::Radar(db) => Some(radar::render_radar(db, config)),
        Diagram::Requirement(db) => Some(requirement::render_requirement(db, config)),
        _ => None,
    }
}

fn render_tertiary_diagram(diagram: &Diagram, config: &RenderConfig) -> Option<Result<String>> {
    match diagram {
        Diagram::Sankey(db) => Some(sankey::render_sankey(db, config)),
        Diagram::Sequence(db) => Some(sequence::render_sequence(db, config)),
        Diagram::State(db) => Some(state::render_state(db, config)),
        Diagram::Timeline(db) => Some(timeline::render_timeline(db, config)),
        Diagram::Treemap(db) => Some(treemap::render_treemap(db, config)),
        Diagram::XyChart(db) => Some(xychart::render_xychart(db, config)),
        _ => None,
    }
}

/// Get the name of the diagram type for error messages
fn diagram_type_name(diagram: &Diagram) -> &'static str {
    primary_diagram_type_name(diagram)
        .or_else(|| secondary_diagram_type_name(diagram))
        .unwrap_or_else(|| tertiary_diagram_type_name(diagram))
}

fn primary_diagram_type_name(diagram: &Diagram) -> Option<&'static str> {
    match diagram {
        Diagram::Architecture(_) => Some("Architecture"),
        Diagram::Block(_) => Some("Block"),
        Diagram::C4(_) => Some("C4"),
        Diagram::Class(_) => Some("Class"),
        Diagram::Er(_) => Some("ER"),
        Diagram::Flowchart(_) => Some("Flowchart"),
        Diagram::Gantt(_) => Some("Gantt"),
        Diagram::Git(_) => Some("Git"),
        _ => None,
    }
}

fn secondary_diagram_type_name(diagram: &Diagram) -> Option<&'static str> {
    match diagram {
        Diagram::Info(_) => Some("Info"),
        Diagram::Journey(_) => Some("Journey"),
        Diagram::Kanban(_) => Some("Kanban"),
        Diagram::Mindmap(_) => Some("Mindmap"),
        Diagram::Packet(_) => Some("Packet"),
        Diagram::Pie(_) => Some("Pie"),
        Diagram::Quadrant(_) => Some("Quadrant"),
        Diagram::Radar(_) => Some("Radar"),
        Diagram::Requirement(_) => Some("Requirement"),
        _ => None,
    }
}

fn tertiary_diagram_type_name(diagram: &Diagram) -> &'static str {
    match diagram {
        Diagram::Sankey(_) => "Sankey",
        Diagram::Sequence(_) => "Sequence",
        Diagram::State(_) => "State",
        Diagram::Timeline(_) => "Timeline",
        Diagram::Treemap(_) => "Treemap",
        Diagram::XyChart(_) => "XyChart",
        _ => "Unknown",
    }
}

/// Render a flowchart diagram
fn render_flowchart(
    db: &crate::diagrams::flowchart::FlowchartDb,
    config: &RenderConfig,
) -> Result<String> {
    let span = tracing::trace_span!(
        "selkie.render.flowchart",
        nodes = db.vertices().len() as u64,
        edges = db.edges().len() as u64,
        direction = db.get_direction(),
        output_bytes = tracing::field::Empty,
    );
    let _enter = span.enter();
    let size_estimator = CharacterSizeEstimator::default();

    // Convert to layout graph
    let graph = {
        let span = tracing::trace_span!("selkie.render.flowchart.to_layout_graph");
        let _enter = span.enter();
        db.to_layout_graph(&size_estimator)?
    };

    // Run layout algorithm
    let graph = {
        let span = tracing::trace_span!(
            "selkie.render.flowchart.layout",
            nodes = graph.all_node_ids().len() as u64,
            edges = graph.edges.len() as u64,
        );
        let _enter = span.enter();
        layout::layout(graph)?
    };

    // Render to SVG
    let renderer = SvgRenderer::new(config.clone());
    let svg = {
        let span = tracing::trace_span!("selkie.render.flowchart.svg");
        let _enter = span.enter();
        renderer.render_flowchart(db, &graph)?
    };
    span.record("output_bytes", svg.len() as u64);
    Ok(svg)
}

/// Render a diagram to ASCII character art.
///
/// This is the primary entry point for ASCII rendering. It accepts any parsed
/// `Diagram` and dispatches to the appropriate type-specific ASCII renderer.
///
/// # Example
///
/// ```
/// let diagram = selkie::parse("flowchart TD\n    A[Start] --> B[End]").unwrap();
/// let ascii = selkie::render::render_ascii(&diagram).unwrap();
/// assert!(ascii.contains("Start"));
/// ```
pub fn render_ascii(diagram: &Diagram) -> Result<String> {
    render_ascii_with_config(diagram, &ascii::AsciiRenderConfig::default())
}

/// Render a diagram to ASCII character art with configuration.
///
/// Like [`render_ascii`], but accepts an [`AsciiRenderConfig`](ascii::AsciiRenderConfig)
/// to control output constraints such as maximum width.
///
/// # Example
///
/// ```
/// use selkie::render::ascii::AsciiRenderConfig;
///
/// let diagram = selkie::parse("flowchart TD\n    A[Start] --> B[End]").unwrap();
/// let config = AsciiRenderConfig { max_width: Some(60), ..Default::default() };
/// let ascii = selkie::render::render_ascii_with_config(&diagram, &config).unwrap();
/// assert!(ascii.lines().all(|line| line.len() <= 60));
/// ```
pub fn render_ascii_with_config(
    diagram: &Diagram,
    config: &ascii::AsciiRenderConfig,
) -> Result<String> {
    let estimator = CharacterSizeEstimator::default();

    let result = render_layout_ascii(diagram, config, &estimator)
        .or_else(|| render_chart_ascii(diagram))
        .or_else(|| render_board_ascii(diagram))
        .unwrap_or_else(|| {
            Err(MermaidError::RenderError(
                "ASCII format not yet supported for this diagram type".to_string(),
            ))
        })?;

    // For diagram types that don't yet thread config internally,
    // apply max_width truncation at the output level.
    Ok(truncate_ascii_width(&result, config))
}

fn render_layout_ascii(
    diagram: &Diagram,
    config: &ascii::AsciiRenderConfig,
    estimator: &CharacterSizeEstimator,
) -> Option<Result<String>> {
    match diagram {
        Diagram::Flowchart(db) => Some(render_flowchart_ascii_configured(db, config, estimator)),
        Diagram::Class(db) => Some(render_class_ascii_configured(db, estimator)),
        Diagram::State(db) => Some(render_layout_graph_ascii_configured(db, config, estimator)),
        Diagram::Er(db) => Some(render_er_ascii_configured(db, estimator)),
        Diagram::Architecture(db) => {
            Some(render_architecture_ascii_configured(db, config, estimator))
        }
        Diagram::Requirement(db) => {
            Some(render_layout_graph_ascii_configured(db, config, estimator))
        }
        _ => None,
    }
}

fn render_flowchart_ascii_configured(
    db: &crate::diagrams::flowchart::FlowchartDb,
    config: &ascii::AsciiRenderConfig,
    estimator: &CharacterSizeEstimator,
) -> Result<String> {
    let graph = db.to_layout_graph(estimator)?;
    let graph = layout::layout(graph)?;
    ascii::render_flowchart_ascii_with_config(db, &graph, config)
}

fn render_class_ascii_configured(
    db: &crate::diagrams::class::ClassDb,
    estimator: &CharacterSizeEstimator,
) -> Result<String> {
    let graph = db.to_layout_graph(estimator)?;
    let graph = layout::layout(graph)?;
    ascii::render_class_ascii(db, &graph)
}

fn render_er_ascii_configured(
    db: &crate::diagrams::er::ErDb,
    estimator: &CharacterSizeEstimator,
) -> Result<String> {
    let graph = db.to_layout_graph(estimator)?;
    let graph = layout::layout(graph)?;
    ascii::render_er_ascii(db, &graph)
}

fn render_layout_graph_ascii_configured<T>(
    db: &T,
    config: &ascii::AsciiRenderConfig,
    estimator: &CharacterSizeEstimator,
) -> Result<String>
where
    T: ToLayoutGraph,
{
    let graph = db.to_layout_graph(estimator)?;
    let graph = layout::layout(graph)?;
    ascii::render_graph_ascii_with_config(&graph, config)
}

fn render_architecture_ascii_configured(
    db: &crate::diagrams::architecture::ArchitectureDb,
    config: &ascii::AsciiRenderConfig,
    estimator: &CharacterSizeEstimator,
) -> Result<String> {
    let graph = architecture::layout_architecture(db, estimator)?;
    ascii::render_graph_ascii_with_config(&graph, config)
}

fn render_chart_ascii(diagram: &Diagram) -> Option<Result<String>> {
    match diagram {
        Diagram::Sequence(db) => Some(ascii::render_sequence_ascii(db)),
        Diagram::Pie(db) => Some(ascii::pie::render_pie_ascii(db)),
        Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            Some(ascii::gantt::render_gantt_ascii(&mut db_clone))
        }
        Diagram::Mindmap(db) => Some(ascii::mindmap::render_mindmap_ascii(db)),
        Diagram::Journey(db) => Some(ascii::journey::render_journey_ascii(db)),
        Diagram::Timeline(db) => Some(ascii::timeline::render_timeline_ascii(db)),
        Diagram::Kanban(db) => Some(ascii::kanban::render_kanban_ascii(db)),
        Diagram::Packet(db) => Some(ascii::packet::render_packet_ascii(db)),
        _ => None,
    }
}

fn render_board_ascii(diagram: &Diagram) -> Option<Result<String>> {
    match diagram {
        Diagram::XyChart(db) => Some(ascii::xychart::render_xychart_ascii(db)),
        Diagram::Quadrant(db) => Some(ascii::quadrant::render_quadrant_ascii(db)),
        Diagram::Radar(db) => Some(ascii::radar::render_radar_ascii(db)),
        Diagram::Git(db) => Some(ascii::gitgraph::render_gitgraph_ascii(db)),
        Diagram::Sankey(db) => Some(ascii::sankey::render_sankey_ascii(db)),
        Diagram::Block(db) => Some(ascii::block::render_block_ascii(db)),
        Diagram::C4(db) => Some(ascii::c4::render_c4_ascii(db)),
        Diagram::Treemap(db) => Some(ascii::treemap::render_treemap_ascii(db)),
        _ => None,
    }
}

/// Truncate each line of ASCII output to the configured max_width.
fn truncate_ascii_width(output: &str, config: &ascii::AsciiRenderConfig) -> String {
    match config.max_width {
        Some(max_w) if max_w > 0 => {
            let mut result = String::with_capacity(output.len());
            for line in output.split('\n') {
                let char_count = line.chars().count();
                if char_count > max_w {
                    let truncated: String = line.chars().take(max_w).collect();
                    result.push_str(&truncated);
                } else {
                    result.push_str(line);
                }
                result.push('\n');
            }
            // Remove trailing extra newline if original didn't end with double newline
            if !output.ends_with("\n\n") && result.ends_with("\n\n") {
                result.pop();
            }
            if output.is_empty() {
                result.clear();
            }
            result
        }
        _ => output.to_string(),
    }
}

/// Render mermaid text directly to ASCII character art.
///
/// This is a convenience function that parses the input text and renders it
/// to ASCII in one step, similar to how [`render_text`] works for SVG.
///
/// # Example
///
/// ```
/// let ascii = selkie::render::render_text_ascii("flowchart TD\n    A[Start] --> B[End]").unwrap();
/// assert!(ascii.contains("Start"));
/// ```
pub fn render_text_ascii(text: &str) -> Result<String> {
    render_text_ascii_with_config(text, &ascii::AsciiRenderConfig::default())
}

/// Render mermaid text directly to ASCII character art with configuration.
///
/// Like [`render_text_ascii`], but accepts an [`AsciiRenderConfig`](ascii::AsciiRenderConfig)
/// for output constraints.
///
/// # Example
///
/// ```
/// use selkie::render::ascii::AsciiRenderConfig;
///
/// let config = AsciiRenderConfig { max_width: Some(80), ..Default::default() };
/// let ascii = selkie::render::render_text_ascii_with_config(
///     "flowchart TD\n    A[Start] --> B[End]",
///     &config,
/// ).unwrap();
/// assert!(ascii.lines().all(|line| line.len() <= 80));
/// ```
pub fn render_text_ascii_with_config(
    text: &str,
    config: &ascii::AsciiRenderConfig,
) -> Result<String> {
    let clean_text = remove_directives(text);
    let diagram_type = detect_type(&clean_text)?;
    let diagram = parse(diagram_type, &clean_text)?;
    render_ascii_with_config(&diagram, config)
}

/// Render an architecture diagram
fn render_architecture(
    db: &crate::diagrams::architecture::ArchitectureDb,
    config: &RenderConfig,
) -> Result<String> {
    let size_estimator = CharacterSizeEstimator::default();

    let graph = architecture::layout_architecture(db, &size_estimator)?;

    let renderer = SvgRenderer::new(config.clone());
    renderer.render_architecture(db, &graph)
}
