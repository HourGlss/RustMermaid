//! Editable graph model for live diagram tooling.
//!
//! This module starts with flowcharts because they are the first target for the
//! large graph editor. The JSON shape is intentionally stable and small enough
//! for browser code to mutate without reparsing Mermaid text for every edit.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::diagrams::flowchart::{
    self, EdgeStroke, FlowClass, FlowText, FlowVertexType, FlowchartDb,
};
use crate::diagrams::{detect_type, remove_directives, DiagramType};
use crate::error::{MermaidError, Result};
use crate::layout::{self, CharacterSizeEstimator, LayoutGraph, LayoutNode, ToLayoutGraph};
use crate::render;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableDiagram {
    pub diagram_type: String,
    pub direction: String,
    #[serde(default)]
    pub nodes: Vec<EditableNode>,
    #[serde(default)]
    pub edges: Vec<EditableEdge>,
    #[serde(default)]
    pub classes: Vec<EditableClass>,
    #[serde(default)]
    pub subgraphs: Vec<EditableSubgraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableNode {
    pub id: String,
    pub label: String,
    pub shape: String,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<EditablePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditablePosition {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub edge_type: String,
    #[serde(default)]
    pub stroke: String,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditablePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableClass {
    pub id: String,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default)]
    pub text_styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableSubgraph {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditablePatch {
    AddNode {
        node: EditableNode,
    },
    RemoveNode {
        id: String,
    },
    AddEdge {
        edge: EditableEdge,
    },
    RemoveEdge {
        id: String,
    },
    MoveNode {
        id: String,
        x: f64,
        y: f64,
        #[serde(default)]
        locked: bool,
    },
    SetNodeLabel {
        id: String,
        label: String,
    },
    SetEdgeLabel {
        id: String,
        label: String,
    },
    SetNodeColor {
        id: String,
        color: String,
    },
    SetEdgeColor {
        id: String,
        color: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditablePatchResult {
    pub graph: EditableDiagram,
    #[serde(default)]
    pub affected_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableRenderParts {
    #[serde(default)]
    pub nodes: Vec<EditableNodePart>,
    #[serde(default)]
    pub edges: Vec<EditableEdgePart>,
    #[serde(default)]
    pub labels: Vec<EditableLabelPart>,
    #[serde(default)]
    pub bounds: Option<EditableBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableNodePart {
    pub id: String,
    pub node_id: String,
    pub label: String,
    pub shape: String,
    pub bounds: EditableBounds,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableEdgePart {
    pub id: String,
    pub edge_id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub points: Vec<EditablePoint>,
    #[serde(default)]
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableLabelPart {
    pub id: String,
    pub owner_id: String,
    pub text: String,
    pub position: EditablePoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditableEdgeRoutes {
    pub node_id: String,
    #[serde(default)]
    pub edges: Vec<EditableEdgePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SelkieMetadata {
    #[serde(default)]
    layout: SelkieLayoutMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SelkieLayoutMetadata {
    #[serde(default)]
    nodes: HashMap<String, EditablePosition>,
}

pub fn parse_to_graph_json(text: &str) -> Result<String> {
    let graph = parse_to_graph(text)?;
    to_pretty_json(&graph)
}

pub fn graph_to_mermaid_text_json(graph_json: &str) -> Result<String> {
    let graph = graph_from_json(graph_json)?;
    graph_to_mermaid_text(&graph)
}

pub fn render_graph_json(graph_json: &str) -> Result<String> {
    let text = graph_to_mermaid_text_json(graph_json)?;
    render::render_text(&text)
}

pub fn layout_graph_json(graph_json: &str) -> Result<String> {
    let graph = graph_from_json(graph_json)?;
    let graph = layout_editable_graph(&graph)?;
    to_pretty_json(&graph)
}

pub fn render_graph_parts_json(graph_json: &str) -> Result<String> {
    let graph = graph_from_json(graph_json)?;
    let parts = render_graph_parts(&graph)?;
    to_pretty_json(&parts)
}

pub fn route_edges_for_node_json(graph_json: &str, node_id: &str) -> Result<String> {
    let graph = graph_from_json(graph_json)?;
    let routes = route_edges_for_node(&graph, node_id)?;
    to_pretty_json(&routes)
}

pub fn apply_graph_patch_json(graph_json: &str, patch_json: &str) -> Result<String> {
    let result = apply_graph_patch_result(graph_json, patch_json)?;
    to_pretty_json(&result.graph)
}

pub fn apply_graph_patch_result_json(graph_json: &str, patch_json: &str) -> Result<String> {
    let result = apply_graph_patch_result(graph_json, patch_json)?;
    to_pretty_json(&result)
}

fn apply_graph_patch_result(graph_json: &str, patch_json: &str) -> Result<EditablePatchResult> {
    let mut graph = graph_from_json(graph_json)?;
    let patch: EditablePatch = serde_json::from_str(patch_json).map_err(json_error)?;
    let affected_ids = affected_render_ids(&graph, &patch);
    apply_graph_patch(&mut graph, patch);
    Ok(EditablePatchResult {
        graph,
        affected_ids,
    })
}

pub fn parse_to_graph(text: &str) -> Result<EditableDiagram> {
    let (metadata, cleaned_text) = extract_selkie_metadata(text);
    let clean_text = remove_directives(&cleaned_text);
    let diagram_type = detect_type(&clean_text)?;
    if diagram_type != DiagramType::Flowchart {
        return Err(MermaidError::InvalidValue {
            message: "editable graph JSON currently supports flowcharts only".to_string(),
        });
    }

    let db = flowchart::parse(&clean_text)?;
    let mut graph = EditableDiagram::from_flowchart_db(&db);
    if let Some(metadata) = metadata {
        graph.apply_metadata(metadata);
    }
    Ok(graph)
}

pub fn graph_to_mermaid_text(graph: &EditableDiagram) -> Result<String> {
    if graph.diagram_type != "flowchart" {
        return Err(MermaidError::InvalidValue {
            message: format!("unsupported editable diagram type '{}'", graph.diagram_type),
        });
    }

    let mut lines = Vec::new();
    if let Some(metadata) = graph.selkie_metadata() {
        lines.push(format!(
            "%%{{selkie: {}}}%%",
            serde_json::to_string(&metadata).map_err(json_error)?
        ));
    }
    lines.push(format!("flowchart {}", graph.direction));

    for class_def in &graph.classes {
        if !class_def.styles.is_empty() {
            lines.push(format!(
                "  classDef {} {}",
                class_def.id,
                class_def.styles.join(",")
            ));
        }
    }

    for node in &graph.nodes {
        lines.push(format!("  {}", node_declaration(node)));
    }

    for edge in &graph.edges {
        lines.push(format!("  {}", edge_declaration(edge)));
    }

    for node in &graph.nodes {
        if !node.classes.is_empty() {
            lines.push(format!("  class {} {}", node.id, node.classes.join(",")));
        }
        if !node.styles.is_empty() {
            lines.push(format!("  style {} {}", node.id, node.styles.join(",")));
        }
    }

    for (idx, edge) in graph.edges.iter().enumerate() {
        if !edge.styles.is_empty() {
            lines.push(format!("  linkStyle {} {}", idx, edge.styles.join(",")));
        }
        for class_name in &edge.classes {
            lines.push(format!("  class {} {}", edge.id, class_name));
        }
    }

    Ok(lines.join("\n"))
}

pub fn graph_from_json(graph_json: &str) -> Result<EditableDiagram> {
    serde_json::from_str(graph_json).map_err(json_error)
}

pub fn layout_editable_graph(graph: &EditableDiagram) -> Result<EditableDiagram> {
    let mut graph = graph.clone();
    let layout_graph = layout_graph_for_editable(&graph)?;
    for node in &mut graph.nodes {
        let Some(layout_node) = layout_graph.get_node(&node.id) else {
            continue;
        };
        let locked = node
            .position
            .as_ref()
            .map(|position| position.locked)
            .unwrap_or(false);
        if locked {
            continue;
        }
        if let (Some(x), Some(y)) = (layout_node.x, layout_node.y) {
            node.position = Some(EditablePosition {
                x,
                y,
                locked: false,
            });
        }
    }
    Ok(graph)
}

pub fn render_graph_parts(graph: &EditableDiagram) -> Result<EditableRenderParts> {
    let layout_graph = layout_graph_for_editable(graph)?;
    let nodes = node_render_parts(graph, &layout_graph);
    let edges = edge_render_parts(graph, &layout_graph, None);
    let labels = label_render_parts(graph, &layout_graph);
    Ok(EditableRenderParts {
        nodes,
        edges,
        labels,
        bounds: graph_bounds(&layout_graph),
    })
}

pub fn route_edges_for_node(graph: &EditableDiagram, node_id: &str) -> Result<EditableEdgeRoutes> {
    let layout_graph = layout_graph_for_editable(graph)?;
    let edges = edge_render_parts(graph, &layout_graph, Some(node_id));
    Ok(EditableEdgeRoutes {
        node_id: node_id.to_string(),
        edges,
    })
}

pub fn flowchart_db_from_graph(graph: &EditableDiagram) -> Result<FlowchartDb> {
    if graph.diagram_type != "flowchart" {
        return Err(MermaidError::InvalidValue {
            message: format!("unsupported editable diagram type '{}'", graph.diagram_type),
        });
    }

    let mut db = FlowchartDb::new();
    db.set_direction(&graph.direction);

    for class_def in &graph.classes {
        db.add_class(&class_def.id, &class_def.styles);
    }

    for node in &graph.nodes {
        db.add_vertex(
            &node.id,
            Some(FlowText::new(node.label.clone())),
            Some(shape_to_vertex_type(&node.shape)?),
            node.styles.clone(),
            node.classes.clone(),
            None,
            None,
        );
    }

    for edge in &graph.edges {
        let arrow = arrow_syntax(edge);
        db.add_edge(
            &edge.source,
            &edge.target,
            &arrow,
            (!edge.label.is_empty()).then_some(edge.label.as_str()),
            Some(&edge.id),
        );
        let edge_idx = db.edges().len().saturating_sub(1);
        if !edge.styles.is_empty() {
            db.set_link_style(edge_idx, &edge.styles);
        }
        for class_name in &edge.classes {
            db.set_class(&edge.id, class_name);
        }
    }

    Ok(db)
}

fn layout_graph_for_editable(graph: &EditableDiagram) -> Result<LayoutGraph> {
    let db = flowchart_db_from_graph(graph)?;
    let size_estimator = CharacterSizeEstimator::default();
    let layout_graph = db.to_layout_graph(&size_estimator)?;
    let mut layout_graph = layout::layout(layout_graph)?;
    apply_locked_positions(graph, &mut layout_graph);
    Ok(layout_graph)
}

fn apply_locked_positions(graph: &EditableDiagram, layout_graph: &mut LayoutGraph) {
    for node in &graph.nodes {
        let Some(position) = node.position.as_ref().filter(|position| position.locked) else {
            continue;
        };
        if let Some(layout_node) = layout_graph.get_node_mut(&node.id) {
            layout_node.x = Some(position.x);
            layout_node.y = Some(position.y);
        }
    }
    layout_graph.compute_bounds();
}

fn node_render_parts(graph: &EditableDiagram, layout_graph: &LayoutGraph) -> Vec<EditableNodePart> {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            let layout_node = layout_graph.get_node(&node.id)?;
            Some(EditableNodePart {
                id: node_render_id(&node.id),
                node_id: node.id.clone(),
                label: node.label.clone(),
                shape: node.shape.clone(),
                bounds: node_bounds(layout_node)?,
                classes: node.classes.clone(),
                styles: node.styles.clone(),
            })
        })
        .collect()
}

fn edge_render_parts(
    graph: &EditableDiagram,
    layout_graph: &LayoutGraph,
    incident_node: Option<&str>,
) -> Vec<EditableEdgePart> {
    graph
        .edges
        .iter()
        .filter(|edge| {
            incident_node
                .map(|node_id| edge.source == node_id || edge.target == node_id)
                .unwrap_or(true)
        })
        .map(|edge| EditableEdgePart {
            id: edge_render_id(&edge.id),
            edge_id: edge.id.clone(),
            source: edge.source.clone(),
            target: edge.target.clone(),
            points: route_points(edge, layout_graph),
            styles: edge.styles.clone(),
        })
        .collect()
}

fn label_render_parts(
    graph: &EditableDiagram,
    layout_graph: &LayoutGraph,
) -> Vec<EditableLabelPart> {
    let mut labels = Vec::new();
    labels.extend(node_label_parts(graph, layout_graph));
    labels.extend(edge_label_parts(graph, layout_graph));
    labels
}

fn node_label_parts(graph: &EditableDiagram, layout_graph: &LayoutGraph) -> Vec<EditableLabelPart> {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            let center = node_center(layout_graph.get_node(&node.id)?)?;
            Some(EditableLabelPart {
                id: node_label_render_id(&node.id),
                owner_id: node_render_id(&node.id),
                text: node.label.clone(),
                position: center,
            })
        })
        .collect()
}

fn edge_label_parts(graph: &EditableDiagram, layout_graph: &LayoutGraph) -> Vec<EditableLabelPart> {
    graph
        .edges
        .iter()
        .filter(|edge| !edge.label.is_empty())
        .filter_map(|edge| {
            let layout_edge = layout_graph.edges.iter().find(|item| item.id == edge.id);
            let position = layout_edge
                .and_then(|item| item.label_position.map(editable_point))
                .or_else(|| route_midpoint(edge, layout_graph))?;
            Some(EditableLabelPart {
                id: edge_label_render_id(&edge.id),
                owner_id: edge_render_id(&edge.id),
                text: edge.label.clone(),
                position,
            })
        })
        .collect()
}

fn route_points(edge: &EditableEdge, layout_graph: &LayoutGraph) -> Vec<EditablePoint> {
    let Some(source) = layout_graph.get_node(&edge.source).and_then(node_center) else {
        return Vec::new();
    };
    let Some(target) = layout_graph.get_node(&edge.target).and_then(node_center) else {
        return Vec::new();
    };
    vec![source, target]
}

fn route_midpoint(edge: &EditableEdge, layout_graph: &LayoutGraph) -> Option<EditablePoint> {
    let points = route_points(edge, layout_graph);
    match points.as_slice() {
        [source, target] => Some(EditablePoint {
            x: (source.x + target.x) / 2.0,
            y: (source.y + target.y) / 2.0,
        }),
        _ => None,
    }
}

fn node_bounds(node: &LayoutNode) -> Option<EditableBounds> {
    Some(EditableBounds {
        x: node.x?,
        y: node.y?,
        width: node.width,
        height: node.height,
    })
}

fn node_center(node: &LayoutNode) -> Option<EditablePoint> {
    Some(EditablePoint {
        x: node.x? + node.width / 2.0,
        y: node.y? + node.height / 2.0,
    })
}

fn graph_bounds(layout_graph: &LayoutGraph) -> Option<EditableBounds> {
    Some(EditableBounds {
        x: layout_graph.bounds_x?,
        y: layout_graph.bounds_y?,
        width: layout_graph.width?,
        height: layout_graph.height?,
    })
}

fn editable_point(point: crate::layout::Point) -> EditablePoint {
    EditablePoint {
        x: point.x,
        y: point.y,
    }
}

fn affected_render_ids(graph: &EditableDiagram, patch: &EditablePatch) -> Vec<String> {
    let mut ids = match patch {
        EditablePatch::AddNode { node } => node_affected_ids(&node.id),
        EditablePatch::RemoveNode { id } => node_and_incident_ids(graph, id),
        EditablePatch::AddEdge { edge } => edge_affected_ids(&edge.id),
        EditablePatch::RemoveEdge { id } => edge_affected_ids(id),
        EditablePatch::MoveNode { id, .. } => node_and_incident_ids(graph, id),
        EditablePatch::SetNodeLabel { id, .. } => node_affected_ids(id),
        EditablePatch::SetEdgeLabel { id, .. } => edge_affected_ids(id),
        EditablePatch::SetNodeColor { id, .. } => vec![node_render_id(id)],
        EditablePatch::SetEdgeColor { id, .. } => vec![edge_render_id(id)],
    };
    ids.sort();
    ids.dedup();
    ids
}

fn node_and_incident_ids(graph: &EditableDiagram, node_id: &str) -> Vec<String> {
    let mut ids = node_affected_ids(node_id);
    for edge in incident_edges(graph, node_id) {
        ids.extend(edge_affected_ids(&edge.id));
    }
    ids
}

fn incident_edges<'a>(
    graph: &'a EditableDiagram,
    node_id: &'a str,
) -> impl Iterator<Item = &'a EditableEdge> {
    graph
        .edges
        .iter()
        .filter(move |edge| edge.source == node_id || edge.target == node_id)
}

fn node_affected_ids(node_id: &str) -> Vec<String> {
    vec![node_render_id(node_id), node_label_render_id(node_id)]
}

fn edge_affected_ids(edge_id: &str) -> Vec<String> {
    vec![edge_render_id(edge_id), edge_label_render_id(edge_id)]
}

fn node_render_id(node_id: &str) -> String {
    format!("node:{node_id}")
}

fn edge_render_id(edge_id: &str) -> String {
    format!("edge:{edge_id}")
}

fn node_label_render_id(node_id: &str) -> String {
    format!("label:node:{node_id}")
}

fn edge_label_render_id(edge_id: &str) -> String {
    format!("label:edge:{edge_id}")
}

pub fn apply_graph_patch(graph: &mut EditableDiagram, patch: EditablePatch) {
    match patch {
        EditablePatch::AddNode { node } => upsert_node(graph, node),
        EditablePatch::RemoveNode { id } => remove_node(graph, &id),
        EditablePatch::AddEdge { edge } => upsert_edge(graph, edge),
        EditablePatch::RemoveEdge { id } => graph.edges.retain(|edge| edge.id != id),
        EditablePatch::MoveNode { id, x, y, locked } => {
            if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
                node.position = Some(EditablePosition { x, y, locked });
            }
        }
        EditablePatch::SetNodeLabel { id, label } => {
            if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
                node.label = label;
            }
        }
        EditablePatch::SetEdgeLabel { id, label } => {
            if let Some(edge) = graph.edges.iter_mut().find(|edge| edge.id == id) {
                edge.label = label;
            }
        }
        EditablePatch::SetNodeColor { id, color } => {
            if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
                upsert_style_property(&mut node.styles, "fill", &color);
            }
        }
        EditablePatch::SetEdgeColor { id, color } => {
            if let Some(edge) = graph.edges.iter_mut().find(|edge| edge.id == id) {
                upsert_style_property(&mut edge.styles, "stroke", &color);
            }
        }
    }
}

impl EditableDiagram {
    pub fn from_flowchart_db(db: &FlowchartDb) -> Self {
        let mut nodes: Vec<EditableNode> = db
            .vertices()
            .values()
            .map(|vertex| EditableNode {
                id: vertex.id.clone(),
                label: vertex.text.clone().unwrap_or_else(|| vertex.id.clone()),
                shape: vertex_type_name(vertex.vertex_type.as_ref()),
                classes: vertex.classes.clone(),
                styles: vertex.styles.clone(),
                position: None,
            })
            .collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let edges = db
            .edges()
            .iter()
            .enumerate()
            .map(|(idx, edge)| EditableEdge {
                id: edge
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("edge-{}-{}-{}", edge.start, edge.end, idx)),
                source: edge.start.clone(),
                target: edge.end.clone(),
                label: edge.text.clone(),
                edge_type: edge.edge_type.clone().unwrap_or_else(|| "-->".to_string()),
                stroke: edge_stroke_name(&edge.stroke),
                classes: edge.classes.clone(),
                styles: edge.style.clone(),
            })
            .collect();

        let mut classes: Vec<EditableClass> =
            db.get_classes().values().map(class_from_flow).collect();
        classes.sort_by(|a, b| a.id.cmp(&b.id));

        let subgraphs = db
            .subgraphs()
            .iter()
            .map(|subgraph| EditableSubgraph {
                id: subgraph.id.clone(),
                title: subgraph.title.clone(),
                nodes: subgraph.nodes.clone(),
                classes: subgraph.classes.clone(),
                direction: subgraph.dir.clone(),
            })
            .collect();

        Self {
            diagram_type: "flowchart".to_string(),
            direction: db.get_direction().to_string(),
            nodes,
            edges,
            classes,
            subgraphs,
        }
    }

    fn apply_metadata(&mut self, metadata: SelkieMetadata) {
        for node in &mut self.nodes {
            if let Some(position) = metadata.layout.nodes.get(&node.id) {
                node.position = Some(position.clone());
            }
        }
    }

    fn selkie_metadata(&self) -> Option<SelkieMetadata> {
        let nodes: HashMap<String, EditablePosition> = self
            .nodes
            .iter()
            .filter_map(|node| {
                node.position
                    .clone()
                    .map(|position| (node.id.clone(), position))
            })
            .collect();

        (!nodes.is_empty()).then_some(SelkieMetadata {
            layout: SelkieLayoutMetadata { nodes },
        })
    }
}

fn class_from_flow(class_def: &FlowClass) -> EditableClass {
    EditableClass {
        id: class_def.id.clone(),
        styles: class_def.styles.clone(),
        text_styles: class_def.text_styles.clone(),
    }
}

fn upsert_node(graph: &mut EditableDiagram, node: EditableNode) {
    if let Some(existing) = graph
        .nodes
        .iter_mut()
        .find(|existing| existing.id == node.id)
    {
        *existing = node;
    } else {
        graph.nodes.push(node);
        graph.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

fn remove_node(graph: &mut EditableDiagram, id: &str) {
    graph.nodes.retain(|node| node.id != id);
    graph
        .edges
        .retain(|edge| edge.source != id && edge.target != id);
}

fn upsert_edge(graph: &mut EditableDiagram, edge: EditableEdge) {
    if let Some(existing) = graph
        .edges
        .iter_mut()
        .find(|existing| existing.id == edge.id)
    {
        *existing = edge;
    } else {
        graph.edges.push(edge);
    }
}

fn upsert_style_property(styles: &mut Vec<String>, property: &str, value: &str) {
    let prefix = format!("{}:", property);
    if let Some(style) = styles.iter_mut().find(|style| style.starts_with(&prefix)) {
        *style = format!("{}:{}", property, value);
    } else {
        styles.push(format!("{}:{}", property, value));
    }
}

fn node_declaration(node: &EditableNode) -> String {
    let label = quoted_label(&node.label);
    match node.shape.as_str() {
        "round" => format!("{}({})", node.id, label),
        "circle" => format!("{}(({}))", node.id, label),
        "double_circle" => format!("{}((({})))", node.id, label),
        "stadium" => format!("{}([{}])", node.id, label),
        "subroutine" => format!("{}[[{}]]", node.id, label),
        "cylinder" => format!("{}[({})]", node.id, label),
        "diamond" => format!("{}{{{}}}", node.id, label),
        "hexagon" => format!("{}{{{{{}}}}}", node.id, label),
        "ellipse" => format!("{}(-{}-)", node.id, label),
        "odd" => format!("{}>{}]", node.id, label),
        "trapezoid" => format!("{}[/{}\\]", node.id, label),
        "inv_trapezoid" => format!("{}[\\{}/]", node.id, label),
        "lean_right" => format!("{}[/{}/]", node.id, label),
        "lean_left" => format!("{}[\\{}\\]", node.id, label),
        _ => format!("{}[{}]", node.id, label),
    }
}

fn edge_declaration(edge: &EditableEdge) -> String {
    let arrow = arrow_syntax(edge);
    let edge_id = if is_mermaid_link_id(&edge.id) {
        format!("{}@", edge.id)
    } else {
        String::new()
    };
    if edge.label.is_empty() {
        format!("{} {}{} {}", edge.source, edge_id, arrow, edge.target)
    } else {
        format!(
            "{} {}{}|{}| {}",
            edge.source,
            edge_id,
            arrow,
            quoted_edge_label(&edge.label),
            edge.target
        )
    }
}

fn is_mermaid_link_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn quoted_label(label: &str) -> String {
    format!("\"{}\"", escape_quoted(label))
}

fn quoted_edge_label(label: &str) -> String {
    escape_quoted(label)
}

fn escape_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn vertex_type_name(vertex_type: Option<&FlowVertexType>) -> String {
    match vertex_type.unwrap_or(&FlowVertexType::Square) {
        FlowVertexType::Square | FlowVertexType::Rect => "square",
        FlowVertexType::DoubleCircle => "double_circle",
        FlowVertexType::Circle => "circle",
        FlowVertexType::Ellipse => "ellipse",
        FlowVertexType::Stadium => "stadium",
        FlowVertexType::Subroutine => "subroutine",
        FlowVertexType::Cylinder => "cylinder",
        FlowVertexType::Round => "round",
        FlowVertexType::Diamond => "diamond",
        FlowVertexType::Hexagon => "hexagon",
        FlowVertexType::Odd => "odd",
        FlowVertexType::Trapezoid => "trapezoid",
        FlowVertexType::InvTrapezoid => "inv_trapezoid",
        FlowVertexType::LeanRight => "lean_right",
        FlowVertexType::LeanLeft => "lean_left",
    }
    .to_string()
}

fn shape_to_vertex_type(shape: &str) -> Result<FlowVertexType> {
    match shape {
        "square" => Ok(FlowVertexType::Square),
        "rect" => Ok(FlowVertexType::Rect),
        "double_circle" => Ok(FlowVertexType::DoubleCircle),
        "circle" => Ok(FlowVertexType::Circle),
        "ellipse" => Ok(FlowVertexType::Ellipse),
        "stadium" => Ok(FlowVertexType::Stadium),
        "subroutine" => Ok(FlowVertexType::Subroutine),
        "cylinder" => Ok(FlowVertexType::Cylinder),
        "round" => Ok(FlowVertexType::Round),
        "diamond" => Ok(FlowVertexType::Diamond),
        "hexagon" => Ok(FlowVertexType::Hexagon),
        "odd" => Ok(FlowVertexType::Odd),
        "trapezoid" => Ok(FlowVertexType::Trapezoid),
        "inv_trapezoid" => Ok(FlowVertexType::InvTrapezoid),
        "lean_right" => Ok(FlowVertexType::LeanRight),
        "lean_left" => Ok(FlowVertexType::LeanLeft),
        other => Err(MermaidError::InvalidValue {
            message: format!("unsupported flowchart node shape '{}'", other),
        }),
    }
}

fn edge_stroke_name(stroke: &EdgeStroke) -> String {
    match stroke {
        EdgeStroke::Normal => "normal",
        EdgeStroke::Thick => "thick",
        EdgeStroke::Invisible => "invisible",
        EdgeStroke::Dotted => "dotted",
    }
    .to_string()
}

fn arrow_for_stroke(stroke: &str) -> &'static str {
    match stroke {
        "thick" => "==>",
        "invisible" => "~~~",
        "dotted" => "-.->",
        _ => "-->",
    }
}

fn arrow_syntax(edge: &EditableEdge) -> String {
    if is_arrow_syntax(&edge.edge_type) {
        return edge.edge_type.clone();
    }
    semantic_arrow_syntax(&edge.edge_type, &edge.stroke)
        .unwrap_or_else(|| arrow_for_stroke(&edge.stroke))
        .to_string()
}

fn is_arrow_syntax(edge_type: &str) -> bool {
    matches!(
        edge_type,
        "-->" | "---" | "==>" | "===" | "-.->" | "-.-" | "<-->"
    )
}

fn semantic_arrow_syntax(edge_type: &str, stroke: &str) -> Option<&'static str> {
    match edge_type {
        "double_arrow_point" => Some(styled_arrow(stroke, "<-->", "<==>", "<-.->")),
        "arrow_open" => Some(open_arrow(stroke)),
        "arrow_cross" => Some(styled_arrow(stroke, "--x", "==x", "-.-x")),
        "arrow_circle" => Some(styled_arrow(stroke, "--o", "==o", "-.-o")),
        "double_arrow_cross" => Some(styled_arrow(stroke, "x--x", "x==x", "x-.-x")),
        "double_arrow_circle" => Some(styled_arrow(stroke, "o--o", "o==o", "o-.-o")),
        _ => None,
    }
}

fn styled_arrow(
    stroke: &str,
    normal: &'static str,
    thick: &'static str,
    dotted: &'static str,
) -> &'static str {
    match stroke {
        "thick" => thick,
        "dotted" => dotted,
        _ => normal,
    }
}

fn open_arrow(stroke: &str) -> &'static str {
    match stroke {
        "thick" => "===",
        "invisible" => "~~~",
        "dotted" => "-.-",
        _ => "---",
    }
}

fn extract_selkie_metadata(text: &str) -> (Option<SelkieMetadata>, String) {
    let Some(start) = text.find("%%{selkie:") else {
        return (None, text.to_string());
    };
    let Some(relative_end) = text[start..].find("}%%") else {
        return (None, text.to_string());
    };

    let directive_end = start + relative_end + 3;
    let json_start = start + "%%{selkie:".len();
    let json_end = start + relative_end;
    let json = text[json_start..json_end].trim();
    let metadata = serde_json::from_str(json).ok();
    let mut cleaned = String::with_capacity(text.len() - (directive_end - start));
    cleaned.push_str(&text[..start]);
    cleaned.push_str(&text[directive_end..]);
    (metadata, cleaned)
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(json_error)
}

fn json_error(err: serde_json::Error) -> MermaidError {
    MermaidError::InvalidValue {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_graph(input: &str) -> EditableDiagram {
        parse_to_graph(input).expect("graph")
    }

    #[test]
    fn flowchart_db_converts_to_stable_graph_json() {
        let graph = parse_graph(
            r#"flowchart TD
  A[Start] --> B{Decision}
  B -->|Yes| C[Done]
  classDef hot fill:#f00,color:#fff
  class A hot
"#,
        );

        assert_eq!(graph.diagram_type, "flowchart");
        assert_eq!(graph.direction, "TB");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.nodes[0].id, "A");
        assert_eq!(graph.nodes[1].shape, "diamond");
        assert_eq!(graph.edges[1].label, "Yes");
        assert_eq!(graph.classes[0].id, "hot");
    }

    #[test]
    fn graph_round_trips_through_mermaid_text() {
        let graph = parse_graph(
            r#"flowchart LR
  A([Start]) -->|go| B{Decision}
  style A fill:#4ecca3
  linkStyle 0 stroke:#e94560
"#,
        );

        let text = graph_to_mermaid_text(&graph).expect("text");
        let reparsed = parse_graph(&text);

        assert_eq!(reparsed.nodes.len(), 2);
        assert_eq!(reparsed.edges.len(), 1);
        assert_eq!(reparsed.nodes[0].styles, vec!["fill:#4ecca3"]);
        assert_eq!(reparsed.edges[0].styles, vec!["stroke:#e94560"]);
        assert_eq!(reparsed.edges[0].label, "go");
    }

    #[test]
    fn render_graph_json_returns_svg() {
        let graph_json =
            parse_to_graph_json("flowchart TD\nA[Start] --> B[End]").expect("graph json");
        let svg = render_graph_json(&graph_json).expect("svg");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Start"));
        assert!(svg.contains("End"));
    }

    #[test]
    fn mutations_update_graph_and_round_trip_to_text() {
        let graph_json =
            parse_to_graph_json("flowchart TD\nA[Start] --> B[End]").expect("graph json");
        let patch = r##"{"op":"add_node","node":{"id":"C","label":"New","shape":"square","classes":[],"styles":[]}}"##;
        let graph_json = apply_graph_patch_json(&graph_json, patch).expect("add node");
        let patch = r##"{"op":"add_edge","edge":{"id":"E1","source":"B","target":"C","label":"next","edge_type":"-->","stroke":"normal","classes":[],"styles":[]}}"##;
        let graph_json = apply_graph_patch_json(&graph_json, patch).expect("add edge");
        let patch = r##"{"op":"set_node_color","id":"C","color":"#ff0000"}"##;
        let graph_json = apply_graph_patch_json(&graph_json, patch).expect("set color");
        let patch = r##"{"op":"set_node_label","id":"C","label":"Created"}"##;
        let graph_json = apply_graph_patch_json(&graph_json, patch).expect("set node label");
        let patch = r##"{"op":"set_edge_color","id":"E1","color":"#00ff00"}"##;
        let graph_json = apply_graph_patch_json(&graph_json, patch).expect("set edge color");

        let text = graph_to_mermaid_text_json(&graph_json).expect("text");
        let reparsed = parse_graph(&text);

        assert!(reparsed.nodes.iter().any(|node| node.id == "C"));
        assert!(reparsed
            .edges
            .iter()
            .any(|edge| edge.source == "B" && edge.target == "C"));
        let node_c = reparsed.nodes.iter().find(|node| node.id == "C").unwrap();
        assert_eq!(node_c.label, "Created");
        assert_eq!(node_c.styles, vec!["fill:#ff0000"]);
        let edge = reparsed.edges.iter().find(|edge| edge.id == "E1").unwrap();
        assert_eq!(edge.styles, vec!["stroke:#00ff00"]);
    }

    #[test]
    fn selkie_position_metadata_round_trips() {
        let graph = parse_graph(
            r#"%%{selkie: {"layout":{"nodes":{"A":{"x":120.0,"y":80.0,"locked":true}}}}}%%
flowchart TD
  A[Start] --> B[End]
"#,
        );
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|node| node.id == "A")
                .unwrap()
                .position,
            Some(EditablePosition {
                x: 120.0,
                y: 80.0,
                locked: true
            })
        );

        let text = graph_to_mermaid_text(&graph).expect("text");
        let reparsed = parse_graph(&text);
        assert_eq!(
            reparsed
                .nodes
                .iter()
                .find(|node| node.id == "A")
                .unwrap()
                .position
                .as_ref()
                .map(|position| position.locked),
            Some(true)
        );
    }

    #[test]
    fn pinned_layout_preserves_locked_node_position() {
        let graph_json = parse_to_graph_json(
            r#"%%{selkie: {"layout":{"nodes":{"A":{"x":120.0,"y":80.0,"locked":true}}}}}%%
flowchart TD
  A[Start] --> B[End]
"#,
        )
        .expect("graph json");

        let laid_out_json = layout_graph_json(&graph_json).expect("layout json");
        let laid_out = graph_from_json(&laid_out_json).expect("laid out graph");
        let node_a = laid_out.nodes.iter().find(|node| node.id == "A").unwrap();
        let node_b = laid_out.nodes.iter().find(|node| node.id == "B").unwrap();

        assert_eq!(
            node_a.position,
            Some(EditablePosition {
                x: 120.0,
                y: 80.0,
                locked: true
            })
        );
        assert!(
            node_b.position.is_some(),
            "unlocked node should receive layout"
        );
    }

    #[test]
    fn render_graph_parts_have_stable_ids() {
        let graph_json =
            parse_to_graph_json("flowchart TD\nA[Start] --> B[End]").expect("graph json");
        let parts_json = render_graph_parts_json(&graph_json).expect("parts json");
        let parts_again_json = render_graph_parts_json(&graph_json).expect("parts json again");
        let parts: EditableRenderParts = serde_json::from_str(&parts_json).unwrap();
        let parts_again: EditableRenderParts = serde_json::from_str(&parts_again_json).unwrap();

        let node_ids: Vec<&str> = parts.nodes.iter().map(|node| node.id.as_str()).collect();
        let edge_ids: Vec<&str> = parts.edges.iter().map(|edge| edge.id.as_str()).collect();
        let label_ids: Vec<&str> = parts.labels.iter().map(|label| label.id.as_str()).collect();

        assert_eq!(parts.nodes.len(), 2);
        assert_eq!(parts.edges.len(), 1);
        assert!(node_ids.contains(&"node:A"));
        assert!(node_ids.contains(&"node:B"));
        assert!(edge_ids.iter().all(|id| id.starts_with("edge:")));
        assert!(label_ids.contains(&"label:node:A"));
        assert_eq!(parts, parts_again);
    }

    #[test]
    fn route_edges_for_node_returns_only_incident_edges() {
        let graph_json =
            parse_to_graph_json("flowchart TD\nA[Start] --> B[Middle]\nB --> C[End]\nA --> C")
                .expect("graph json");
        let graph = graph_from_json(&graph_json).expect("graph");
        let edge_ab = graph
            .edges
            .iter()
            .find(|edge| edge.source == "A" && edge.target == "B")
            .unwrap()
            .id
            .clone();
        let edge_bc = graph
            .edges
            .iter()
            .find(|edge| edge.source == "B" && edge.target == "C")
            .unwrap()
            .id
            .clone();
        let routes_json = route_edges_for_node_json(&graph_json, "A").expect("routes");
        let routes: EditableEdgeRoutes = serde_json::from_str(&routes_json).unwrap();
        let route_ids: Vec<&str> = routes
            .edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect();

        assert_eq!(routes.node_id, "A");
        assert!(route_ids.contains(&edge_ab.as_str()));
        assert!(!route_ids.contains(&edge_bc.as_str()));
        assert!(routes.edges.iter().all(|edge| edge.points.len() == 2));
    }

    #[test]
    fn move_node_patch_reports_affected_render_ids() {
        let graph_json = parse_to_graph_json("flowchart TD\nA[Start] --> B[Middle]\nB --> C[End]")
            .expect("graph json");
        let graph = graph_from_json(&graph_json).expect("graph");
        let edge_ab = graph
            .edges
            .iter()
            .find(|edge| edge.source == "A" && edge.target == "B")
            .unwrap()
            .id
            .clone();
        let edge_bc = graph
            .edges
            .iter()
            .find(|edge| edge.source == "B" && edge.target == "C")
            .unwrap()
            .id
            .clone();
        let patch = r#"{"op":"move_node","id":"A","x":200.0,"y":100.0,"locked":true}"#;
        let result_json = apply_graph_patch_result_json(&graph_json, patch).expect("patch result");
        let result: EditablePatchResult = serde_json::from_str(&result_json).unwrap();

        assert!(result.affected_ids.contains(&"node:A".to_string()));
        assert!(result.affected_ids.contains(&format!("edge:{}", edge_ab)));
        assert!(!result.affected_ids.contains(&format!("edge:{}", edge_bc)));

        let node_a = result
            .graph
            .nodes
            .iter()
            .find(|node| node.id == "A")
            .unwrap();
        assert_eq!(
            node_a.position,
            Some(EditablePosition {
                x: 200.0,
                y: 100.0,
                locked: true
            })
        );
    }
}
