//! SVG structure analysis for comparison testing
//!
//! This module provides tools to analyze SVG documents and extract
//! structural information for comparison between different renderers.

use serde::{Deserialize, Serialize};

/// Structural analysis of an SVG document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SvgStructure {
    /// Width of the SVG (from viewBox or width attribute)
    pub width: f64,
    /// Height of the SVG (from viewBox or height attribute)
    pub height: f64,
    /// Number of node elements detected
    pub node_count: usize,
    /// Number of edge elements detected
    pub edge_count: usize,
    /// Text labels found in the SVG
    pub labels: Vec<String>,
    /// Count of each shape type
    pub shapes: ShapeCounts,
    /// Number of marker definitions
    pub marker_count: usize,
    /// Whether the SVG has a defs section
    pub has_defs: bool,
    /// Whether the SVG has embedded styles
    pub has_style: bool,
    /// Z-order analysis: tracks element rendering order
    pub z_order: ZOrderAnalysis,
    /// Stroke width analysis: tracks stroke-width values on key elements
    pub stroke_analysis: StrokeAnalysis,
    /// Edge geometry analysis: tracks edge endpoint positions
    pub edge_geometry: EdgeGeometry,
    /// Font analysis: tracks font-size and font-weight on text elements
    pub font_analysis: FontAnalysis,
    /// Color analysis: tracks fill and stroke colors used
    pub color_analysis: ColorAnalysis,
    /// Raw SVG string for additional parsing if needed
    pub raw_svg: String,
}

/// Analysis of SVG element rendering order (z-order)
/// In SVG, later elements are drawn on top of earlier ones
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ZOrderAnalysis {
    /// Text elements that appear before shapes in the same group (potentially obscured)
    pub text_before_shapes: usize,
    /// Text elements that appear after shapes in the same group (correct order)
    pub text_after_shapes: usize,
    /// Labels that may be obscured (text rendered before overlapping shapes)
    pub potentially_obscured_labels: Vec<String>,
    /// Element order summary: list of (element_type, count) in render order
    pub element_order: Vec<(String, usize)>,
}

/// Counts of different SVG shape elements
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShapeCounts {
    pub rect: usize,
    pub circle: usize,
    pub ellipse: usize,
    pub polygon: usize,
    pub path: usize,
    pub line: usize,
    pub polyline: usize,
}

/// Analysis of stroke-width values across the SVG
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StrokeAnalysis {
    /// Stroke widths found on rect elements (typically entity/node borders)
    pub rect_stroke_widths: Vec<f64>,
    /// Stroke widths found on path elements (typically edges/lines)
    pub path_stroke_widths: Vec<f64>,
    /// Stroke widths found on line elements
    pub line_stroke_widths: Vec<f64>,
    /// Average stroke width on rects (0 if none)
    pub avg_rect_stroke: f64,
    /// Average stroke width on paths (0 if none)
    pub avg_path_stroke: f64,
}

/// Analysis of colors used in the SVG
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ColorAnalysis {
    /// Unique fill colors found (normalized to lowercase)
    pub fill_colors: Vec<String>,
    /// Unique stroke colors found (normalized to lowercase)
    pub stroke_colors: Vec<String>,
    /// Count of elements with fill
    pub fill_count: usize,
    /// Count of elements with stroke
    pub stroke_count: usize,
    /// Text elements with potential visibility issues (CSS fill override)
    pub text_visibility_issues: Vec<TextVisibilityIssue>,
}

/// A text element with potential visibility issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextVisibilityIssue {
    /// The text content
    pub text: String,
    /// The CSS class that defines fill
    pub css_class: String,
    /// The fill color from CSS
    pub css_fill: String,
    /// The inline fill attribute (if any)
    pub inline_fill: Option<String>,
    /// The background fill color (from parent or sibling rect)
    pub background_fill: Option<String>,
}

/// Analysis of edge/path geometry
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EdgeGeometry {
    /// Edge endpoints: list of (start_x, start_y, end_x, end_y)
    pub edge_endpoints: Vec<(f64, f64, f64, f64)>,
    /// Initial direction points for each edge (the second point in the path)
    /// Used to determine the initial tangent direction for curved paths
    pub edge_initial_directions: Vec<Option<(f64, f64)>>,
    /// Node bounding boxes: list of (x, y, width, height, id/class)
    pub node_bounds: Vec<NodeBounds>,
    /// Text bounding boxes with their content and parent info
    pub text_bounds: Vec<TextBounds>,
    /// Edges that attach to top/bottom of nodes (vertical attachment)
    pub vertical_attachments: usize,
    /// Edges that attach to left/right of nodes (horizontal attachment)
    pub horizontal_attachments: usize,
    /// Detailed edge attachment information
    pub edge_details: Vec<EdgeDetail>,
}

/// Detailed information about a single edge
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EdgeDetail {
    /// Start point coordinates
    pub start: (f64, f64),
    /// End point coordinates
    pub end: (f64, f64),
    /// Node ID at start (if identified)
    pub start_node: Option<String>,
    /// Node ID at end (if identified)
    pub end_node: Option<String>,
    /// Which edge of the start node (top, bottom, left, right)
    pub start_edge: String,
    /// Which edge of the end node (top, bottom, left, right)
    pub end_edge: String,
    /// Offset from center of start edge (0 = centered)
    pub start_center_offset: f64,
    /// Offset from center of end edge (0 = centered)
    pub end_center_offset: f64,
}

/// Bounding box of a node element
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub id: String,
}

/// Bounding box of a text element with its content
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The text content
    pub text: String,
    /// Parent node ID if the text is inside a node
    pub parent_node_id: Option<String>,
}

/// Analysis of font styles used in text elements
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FontAnalysis {
    /// Font sizes found (class/context -> size)
    pub font_sizes: Vec<FontStyle>,
    /// Font weights found (class/context -> weight)
    pub font_weights: Vec<FontStyle>,
    /// Count of text elements analyzed
    pub text_count: usize,
}

/// A font style value with its context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontStyle {
    /// CSS class or context where this style was found
    pub context: String,
    /// The value (e.g., "14" for font-size, "bold" for font-weight)
    pub value: String,
}

impl SvgStructure {
    /// Parse an SVG string and extract its structure
    pub fn from_svg(svg: &str) -> Result<Self, String> {
        let doc =
            roxmltree::Document::parse(svg).map_err(|e| format!("Failed to parse SVG: {}", e))?;

        let root = doc.root_element();
        if root.tag_name().name() != "svg" {
            return Err("Root element is not <svg>".to_string());
        }

        // Parse dimensions
        let (width, height) = parse_dimensions(&root);

        // Count shapes
        let shapes = count_shapes(&doc);

        // Count nodes and edges (elements with specific classes)
        let (node_count, edge_count) = count_nodes_and_edges(&doc);

        // Extract labels
        let labels = extract_labels(&doc);

        // Count markers
        let marker_count = count_elements(&doc, "marker");

        // Check for defs and style
        let has_defs = doc.descendants().any(|n| n.tag_name().name() == "defs");
        let has_style = doc.descendants().any(|n| n.tag_name().name() == "style");

        // Analyze z-order (element rendering order)
        let z_order = analyze_z_order(&doc);

        // Analyze stroke widths
        let stroke_analysis = analyze_stroke_widths(&doc);

        // Analyze edge geometry
        let edge_geometry = analyze_edge_geometry(&doc);

        // Analyze font styles
        let font_analysis = analyze_fonts(&doc);

        // Analyze colors
        let color_analysis = analyze_colors(&doc);

        Ok(SvgStructure {
            width,
            height,
            node_count,
            edge_count,
            labels,
            shapes,
            marker_count,
            has_defs,
            has_style,
            z_order,
            stroke_analysis,
            edge_geometry,
            font_analysis,
            color_analysis,
            raw_svg: svg.to_string(),
        })
    }
}

// Helper functions

fn parse_dimensions(root: &roxmltree::Node) -> (f64, f64) {
    // Try viewBox first
    if let Some(viewbox) = root.attribute("viewBox") {
        let parts: Vec<f64> = viewbox
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 4 {
            return (parts[2], parts[3]);
        }
    }

    // Fall back to width/height attributes
    let width = root
        .attribute("width")
        .and_then(|s| s.trim_end_matches("px").parse().ok())
        .unwrap_or(0.0);
    let height = root
        .attribute("height")
        .and_then(|s| s.trim_end_matches("px").parse().ok())
        .unwrap_or(0.0);

    (width, height)
}

fn count_shapes(doc: &roxmltree::Document) -> ShapeCounts {
    ShapeCounts {
        rect: count_visible_rects(doc),
        circle: count_elements(doc, "circle"),
        ellipse: count_elements(doc, "ellipse"),
        polygon: count_elements(doc, "polygon"),
        path: count_visible_paths(doc),
        line: count_elements(doc, "line"),
        polyline: count_elements(doc, "polyline"),
    }
}

/// Count only visible rects (those with width and height > 0)
/// This excludes helper/placeholder rects used by mermaid.js for sizing
/// and edge label background rects (class="edge-label-bg")
fn count_visible_rects(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == "rect")
        .filter(|n| {
            // Exclude edge label backgrounds (not structural elements)
            let class = n.attribute("class").unwrap_or("");
            if class.contains("edge-label-bg") {
                return false;
            }

            // Check if rect has non-zero dimensions
            let width = n
                .attribute("width")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let height = n
                .attribute("height")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            width > 0.0 && height > 0.0
        })
        .count()
}

fn count_elements(doc: &roxmltree::Document, tag: &str) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == tag)
        .count()
}

fn count_visible_paths(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|n| n.tag_name().name() == "path")
        .filter(|n| {
            // Exclude label backgrounds (not structural elements)
            let class = n.attribute("class").unwrap_or("");
            if class.contains("label-bg") {
                return false;
            }

            let stroke = n.attribute("stroke");
            if stroke == Some("none") {
                return false;
            }

            if let Some(width) = n.attribute("stroke-width") {
                if width.parse::<f64>().ok() == Some(0.0) {
                    return false;
                }
            }

            true
        })
        .count()
}

fn count_nodes_and_edges(doc: &roxmltree::Document) -> (usize, usize) {
    let mut node_count = 0;
    let mut edge_count = 0;

    // Node class patterns used by different diagram types in selkie and mermaid.js
    const NODE_CLASSES: &[&str] = &[
        "node",             // flowchart (selkie), mindmap (mermaid.js)
        "flowchart-node",   // flowchart (mermaid.js)
        "class-node",       // class diagram (selkie)
        "state-node",       // state diagram (selkie)
        "entity-node",      // ER diagram (selkie)
        "requirement-node", // requirement diagram (selkie)
        "element-node",     // requirement diagram elements (selkie)
        "mindmap-node",     // mindmap (selkie)
        "architecture-service",
        "architecture-junction",
    ];

    // Edge class patterns used by different diagram types
    const EDGE_CLASSES: &[&str] = &[
        "edge",         // flowchart (selkie)
        "relation",     // class diagram (selkie)
        "transition",   // state diagram (selkie)
        "relationship", // ER diagram (selkie)
    ];

    for node in doc.descendants() {
        // Check for data-edge attribute (mermaid.js uses this)
        if node.attribute("data-edge").is_some() {
            edge_count += 1;
            continue;
        }

        if let Some(class) = node.attribute("class") {
            let classes: Vec<&str> = class.split_whitespace().collect();

            // Count nodes - elements with any node class pattern
            if classes.iter().any(|c| NODE_CLASSES.contains(c)) {
                node_count += 1;
            }

            // Count edges - handle group containers and architecture edge paths
            // mermaid.js uses "flowchart-link" on <path> elements with data-edge
            // (handled above with data-edge attribute check)
            if classes.iter().any(|c| EDGE_CLASSES.contains(c)) {
                let tag = node.tag_name().name();
                if tag == "g" || tag == "path" {
                    edge_count += 1;
                }
            }
        }
    }

    (node_count, edge_count)
}

fn extract_labels(doc: &roxmltree::Document) -> Vec<String> {
    let mut labels = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in doc.descendants() {
        let tag = node.tag_name().name();

        // For text elements, combine all text content into a single label
        if tag == "text" {
            // Combine ALL tspans into a single label (whether multi-line with dy or not)
            // This matches HTML <p> extraction which gets the full text content
            let combined = collect_text_content(&node);
            // Normalize whitespace: collapse multiple spaces/newlines into single space
            let combined: String = combined.split_whitespace().collect::<Vec<_>>().join(" ");
            if !combined.is_empty() && !seen.contains(&combined) {
                seen.insert(combined.clone());
                labels.push(combined);
            }
        }
        // For tspan directly under text, handled above
        // For p/span (mermaid.js foreignObject HTML), collect ALL text content
        // including text after <br> elements, matching how we handle <text> with tspans
        else if tag == "p" || tag == "span" {
            let combined = collect_text_content(&node);
            let combined: String = combined.split_whitespace().collect::<Vec<_>>().join(" ");
            if !combined.is_empty() && !seen.contains(&combined) {
                seen.insert(combined.clone());
                labels.push(combined);
            }
        }
    }

    labels.sort();
    labels
}

/// Recursively collect all text content from a node and its descendants
/// Adds spaces between tspan elements and around <br> tags to ensure proper word boundaries
fn collect_text_content(node: &roxmltree::Node) -> String {
    let mut result = String::new();

    for child in node.children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                result.push_str(text);
            }
        } else {
            let tag = child.tag_name().name();
            // <br>, <tspan>, and other block-like elements act as word boundaries
            if !result.is_empty()
                && !result.ends_with(' ')
                && !result.ends_with('\n')
                && (tag == "tspan" || tag == "br")
            {
                result.push(' ');
            }
            result.push_str(&collect_text_content(&child));
        }
    }

    result
}

/// Analyze z-order (rendering order) of SVG elements
/// In SVG, later elements are rendered on top of earlier ones
fn analyze_z_order(doc: &roxmltree::Document) -> ZOrderAnalysis {
    let mut analysis = ZOrderAnalysis::default();
    let mut element_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Shape element types that could obscure text
    const SHAPE_TAGS: &[&str] = &[
        "rect", "circle", "ellipse", "polygon", "path", "line", "polyline",
    ];
    const TEXT_TAGS: &[&str] = &["text", "tspan", "foreignObject"];

    // Analyze each group (g element) for text/shape ordering
    for group in doc.descendants().filter(|n| n.tag_name().name() == "g") {
        let mut last_shape_index: Option<usize> = None;
        let mut last_text_index: Option<usize> = None;

        for (i, child) in group.children().enumerate() {
            let tag = child.tag_name().name();

            if SHAPE_TAGS.contains(&tag) {
                last_shape_index = Some(i);

                // If text was rendered before this shape, it might be obscured
                if let Some(text_idx) = last_text_index {
                    if text_idx < i {
                        analysis.text_before_shapes += 1;
                        // Try to extract the label that might be obscured
                        if let Some(text_node) = group.children().nth(text_idx) {
                            let label = collect_text_content(&text_node)
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ");
                            if !label.is_empty()
                                && !analysis.potentially_obscured_labels.contains(&label)
                            {
                                analysis.potentially_obscured_labels.push(label);
                            }
                        }
                    }
                }
            }

            if TEXT_TAGS.contains(&tag) {
                last_text_index = Some(i);

                // Check if text comes after shapes (correct order)
                if last_shape_index.is_some() {
                    analysis.text_after_shapes += 1;
                }
            }
        }
    }

    // Build element order summary (top-level elements in the main SVG)
    for node in doc.root_element().children() {
        let tag = node.tag_name().name();
        if !tag.is_empty() {
            *element_counts.entry(tag.to_string()).or_insert(0) += 1;
        }
    }

    // Convert to ordered list
    let mut order: Vec<_> = element_counts.into_iter().collect();
    order.sort_by(|a, b| a.0.cmp(&b.0));
    analysis.element_order = order;

    analysis
}

/// Analyze stroke-width values across the SVG
/// Extracts from both inline attributes and CSS <style> blocks
fn analyze_stroke_widths(doc: &roxmltree::Document) -> StrokeAnalysis {
    let mut analysis = StrokeAnalysis::default();

    // First, extract stroke-width values from CSS <style> blocks
    let css_stroke_widths = extract_css_stroke_widths(doc);

    for node in doc.descendants() {
        let tag = node.tag_name().name();

        // Get stroke-width from inline attribute
        let inline_stroke_width = node
            .attribute("stroke-width")
            .and_then(|s| s.parse::<f64>().ok());

        // Get stroke-width from CSS class or element type selector
        let class = node.attribute("class").unwrap_or("");
        let css_stroke_width = class
            .split_whitespace()
            .find_map(|c| css_stroke_widths.get(c).copied())
            .or_else(|| {
                css_stroke_widths
                    .get(&format!("__element_{}", tag))
                    .copied()
            });

        // Use inline if present, otherwise CSS, otherwise check if has stroke
        let stroke_width = inline_stroke_width.or(css_stroke_width);

        // Skip elements with stroke explicitly set to "none"
        if node.attribute("stroke") == Some("none") {
            continue;
        }

        // Only count if element has a visible stroke
        let has_stroke = node
            .attribute("stroke")
            .map(|s| s != "none")
            .unwrap_or(false)
            || stroke_width.is_some()
            || class
                .split_whitespace()
                .any(|c| css_stroke_widths.contains_key(c));

        if !has_stroke {
            continue;
        }

        let width = stroke_width.unwrap_or(1.0);

        match tag {
            "rect" => analysis.rect_stroke_widths.push(width),
            "path" => analysis.path_stroke_widths.push(width),
            "line" => analysis.line_stroke_widths.push(width),
            _ => {}
        }
    }

    // Calculate averages
    if !analysis.rect_stroke_widths.is_empty() {
        analysis.avg_rect_stroke = analysis.rect_stroke_widths.iter().sum::<f64>()
            / analysis.rect_stroke_widths.len() as f64;
    }
    if !analysis.path_stroke_widths.is_empty() {
        analysis.avg_path_stroke = analysis.path_stroke_widths.iter().sum::<f64>()
            / analysis.path_stroke_widths.len() as f64;
    }

    analysis
}

/// Extract stroke-width values from CSS <style> blocks
/// Returns a map of selector component -> stroke-width value
#[cfg(feature = "eval")]
fn extract_css_stroke_widths(doc: &roxmltree::Document) -> std::collections::HashMap<String, f64> {
    use simplecss::StyleSheet;

    let mut css_strokes = std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                // Parse CSS using simplecss
                let stylesheet = StyleSheet::parse(css_text);

                for rule in stylesheet.rules {
                    // Check if this rule has a stroke-width declaration
                    let mut stroke_width: Option<f64> = None;

                    for decl in &rule.declarations {
                        if decl.name == "stroke-width" {
                            // Parse value, stripping 'px' suffix if present
                            let value = decl.value.trim().trim_end_matches("px");
                            if let Ok(width) = value.parse::<f64>() {
                                stroke_width = Some(width);
                            }
                        }
                    }

                    // If we found a stroke-width, associate it with selector components
                    if let Some(width) = stroke_width {
                        let selector_str = rule.selector.to_string();

                        // Extract class names from selector
                        for part in selector_str.split(&[' ', ',', '>', '+', '~'][..]) {
                            let part = part.trim();
                            if part.starts_with('.') {
                                let class = part.trim_start_matches('.');
                                css_strokes.insert(class.to_string(), width);
                            }
                            // Also track element type selectors
                            match part {
                                "rect" | "path" | "line" | "circle" | "ellipse" => {
                                    css_strokes.insert(format!("__element_{}", part), width);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    css_strokes
}

/// Fallback when eval feature is disabled - returns empty map
#[cfg(not(feature = "eval"))]
fn extract_css_stroke_widths(_doc: &roxmltree::Document) -> std::collections::HashMap<String, f64> {
    std::collections::HashMap::new()
}

/// Analyze edge geometry - endpoints and attachment points
fn analyze_edge_geometry(doc: &roxmltree::Document) -> EdgeGeometry {
    let mut geometry = EdgeGeometry {
        node_bounds: collect_node_bounds(doc),
        ..Default::default()
    };
    collect_edge_paths(doc, &mut geometry);
    geometry.text_bounds = extract_text_bounds(doc, &geometry.node_bounds);

    geometry
}

fn collect_node_bounds(doc: &roxmltree::Document) -> Vec<NodeBounds> {
    let mut node_bounds = Vec::new();

    for node in doc.descendants() {
        node_bounds.extend(node_bounds_from_element(node));
    }

    node_bounds
}

fn node_bounds_from_element(node: roxmltree::Node<'_, '_>) -> Vec<NodeBounds> {
    match node.tag_name().name() {
        "rect" => rect_node_bounds(node).into_iter().collect(),
        "g" => group_node_bounds(node).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn rect_node_bounds(node: roxmltree::Node<'_, '_>) -> Option<NodeBounds> {
    let class = node.attribute("class").unwrap_or("");
    if !is_node_rect_class(class) {
        return None;
    }

    let (offset_x, offset_y) = accumulated_translate(node);
    node_bounds_from_rect(node, offset_x, offset_y, node.attribute("id").unwrap_or(""))
}

fn is_node_rect_class(class: &str) -> bool {
    class.contains("entity-box")
        || class.contains("node")
        || class.contains("actor")
        || class.contains("label-container")
}

fn group_node_bounds(node: roxmltree::Node<'_, '_>) -> Option<NodeBounds> {
    let class = node.attribute("class").unwrap_or("");
    let id = node.attribute("id").unwrap_or("");
    let is_timeline_node = is_timeline_node_class(class);
    let is_architecture_node = is_architecture_node_class(class);

    if !is_node_group(class, id, is_timeline_node, is_architecture_node) {
        return None;
    }

    let (cx, cy) = accumulated_translate(node);
    if cx == 0.0 && cy == 0.0 && node.attribute("transform").is_none() {
        return None;
    }

    group_path_bounds(node, class, id, cx, cy, is_timeline_node)
        .or_else(|| architecture_group_bounds(node, id, cx, cy, is_architecture_node))
        .or_else(|| child_rect_group_bounds(node, id, cx, cy))
}

fn is_timeline_node_class(class: &str) -> bool {
    class.contains("taskWrapper")
        || class.contains("eventWrapper")
        || class.contains("timeline-node")
}

fn is_architecture_node_class(class: &str) -> bool {
    class.contains("architecture-service") || class.contains("architecture-junction")
}

fn is_node_group(
    class: &str,
    id: &str,
    is_timeline_node: bool,
    is_architecture_node: bool,
) -> bool {
    is_timeline_node
        || is_architecture_node
        || (class.contains("node")
            && (id.contains("entity")
                || id.starts_with("block-")
                || id.starts_with("flowchart-")
                || id.starts_with("id-")
                || id.starts_with("id")
                || id.starts_with("node-")))
}

fn group_path_bounds(
    node: roxmltree::Node<'_, '_>,
    class: &str,
    id: &str,
    cx: f64,
    cy: f64,
    is_timeline_node: bool,
) -> Option<NodeBounds> {
    for path_node in group_path_candidates(node, is_timeline_node) {
        if let Some(bounds) = bounds_from_group_path(path_node, class, id, cx, cy, is_timeline_node)
        {
            return Some(bounds);
        }
    }
    None
}

fn group_path_candidates<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    is_timeline_node: bool,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.descendants()
        .filter(|node| node.tag_name().name() == "path")
        .filter(move |node| {
            let path_class = node.attribute("class").unwrap_or("");
            path_class.contains("node-bkg") || !is_timeline_node
        })
        .collect()
}

fn bounds_from_group_path(
    path_node: roxmltree::Node<'_, '_>,
    class: &str,
    id: &str,
    cx: f64,
    cy: f64,
    is_timeline_node: bool,
) -> Option<NodeBounds> {
    let d = path_node.attribute("d")?;
    if let Some((half_w, half_h)) = parse_rect_path_dimensions(d) {
        Some(NodeBounds {
            x: cx - half_w,
            y: cy - half_h,
            width: half_w * 2.0,
            height: half_h * 2.0,
            id: id.to_string(),
        })
    } else if is_timeline_node {
        parse_timeline_path_dimensions(d).map(|(width, height)| NodeBounds {
            x: cx,
            y: cy,
            width,
            height,
            id: if id.is_empty() {
                class.to_string()
            } else {
                id.to_string()
            },
        })
    } else {
        None
    }
}

fn architecture_group_bounds(
    node: roxmltree::Node<'_, '_>,
    id: &str,
    cx: f64,
    cy: f64,
    is_architecture_node: bool,
) -> Option<NodeBounds> {
    if !is_architecture_node {
        return None;
    }

    node.descendants()
        .find_map(|desc| architecture_descendant_bounds(desc, id, cx, cy))
}

fn architecture_descendant_bounds(
    desc: roxmltree::Node<'_, '_>,
    id: &str,
    cx: f64,
    cy: f64,
) -> Option<NodeBounds> {
    let tag = desc.tag_name().name();
    let width = parse_node_attr(desc, "width");
    let height = parse_node_attr(desc, "height");

    if matches!(tag, "svg" | "rect") && width >= 80.0 && height >= 80.0 {
        Some(NodeBounds {
            x: cx,
            y: cy,
            width,
            height,
            id: id.to_string(),
        })
    } else {
        None
    }
}

fn child_rect_group_bounds(
    node: roxmltree::Node<'_, '_>,
    id: &str,
    cx: f64,
    cy: f64,
) -> Option<NodeBounds> {
    node.children()
        .find(|child| child.tag_name().name() == "rect")
        .and_then(|child| node_bounds_from_rect(child, cx, cy, id))
}

fn node_bounds_from_rect(
    node: roxmltree::Node<'_, '_>,
    offset_x: f64,
    offset_y: f64,
    id: &str,
) -> Option<NodeBounds> {
    let width = parse_node_attr(node, "width");
    let height = parse_node_attr(node, "height");

    (width > 0.0 && height > 0.0).then(|| NodeBounds {
        x: offset_x + parse_node_attr(node, "x"),
        y: offset_y + parse_node_attr(node, "y"),
        width,
        height,
        id: id.to_string(),
    })
}

fn parse_node_attr(node: roxmltree::Node<'_, '_>, attr: &str) -> f64 {
    node.attribute(attr)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.0)
}

fn collect_edge_paths(doc: &roxmltree::Document, geometry: &mut EdgeGeometry) {
    for node in doc.descendants().filter(is_edge_path_node) {
        if let Some(d) = node.attribute("d") {
            collect_edge_path(d, accumulated_translate(node), geometry);
        }
    }
}

fn is_edge_path_node(node: &roxmltree::Node<'_, '_>) -> bool {
    if node.tag_name().name() != "path" {
        return false;
    }

    let class = node.attribute("class").unwrap_or("");
    !class.contains("label-bg")
        && (class.contains("relationship")
            || class.contains("relation")
            || class.contains("edge")
            || class.contains("link")
            || class.contains("transition"))
}

fn collect_edge_path(d: &str, offset: (f64, f64), geometry: &mut EdgeGeometry) {
    let Some((start, second_point, end)) = parse_path_with_directions(d) else {
        return;
    };
    let start = translate_point(start, offset);
    let second_point = second_point.map(|point| translate_point(point, offset));
    let end = translate_point(end, offset);

    geometry
        .edge_endpoints
        .push((start.0, start.1, end.0, end.1));
    geometry.edge_initial_directions.push(second_point);
    let (best_start, best_end, vertical_count, horizontal_count) =
        best_edge_attachments(start, end, &geometry.node_bounds);
    geometry.vertical_attachments += vertical_count;
    geometry.horizontal_attachments += horizontal_count;
    geometry
        .edge_details
        .push(edge_detail(start, end, best_start, best_end));
}

fn translate_point(point: (f64, f64), offset: (f64, f64)) -> (f64, f64) {
    (point.0 + offset.0, point.1 + offset.1)
}

fn best_edge_attachments(
    start: (f64, f64),
    end: (f64, f64),
    node_bounds: &[NodeBounds],
) -> (Option<AttachmentInfo>, Option<AttachmentInfo>, usize, usize) {
    let mut best_start = None;
    let mut best_end = None;
    let mut vertical_count = 0;
    let mut horizontal_count = 0;

    for bounds in node_bounds {
        update_best_attachment(&mut best_start, classify_attachment_detailed(start, bounds));
        update_best_attachment(&mut best_end, classify_attachment_detailed(end, bounds));
        let (vertical, horizontal) = attachment_type_counts(start, end, bounds);
        vertical_count += vertical;
        horizontal_count += horizontal;
    }

    (best_start, best_end, vertical_count, horizontal_count)
}

fn update_best_attachment(best: &mut Option<AttachmentInfo>, candidate: AttachmentInfo) {
    if candidate.attach_type == AttachmentType::None {
        return;
    }
    if best
        .as_ref()
        .is_none_or(|current| candidate.distance < current.distance)
    {
        *best = Some(candidate);
    }
}

fn attachment_type_counts(
    start: (f64, f64),
    end: (f64, f64),
    bounds: &NodeBounds,
) -> (usize, usize) {
    let (attach_type_start, _) = classify_attachment(start, bounds);
    let (attach_type_end, _) = classify_attachment(end, bounds);
    let vertical = usize::from(
        attach_type_start == AttachmentType::Vertical
            || attach_type_end == AttachmentType::Vertical,
    );
    let horizontal = usize::from(
        attach_type_start == AttachmentType::Horizontal
            || attach_type_end == AttachmentType::Horizontal,
    );

    (vertical, horizontal)
}

fn edge_detail(
    start: (f64, f64),
    end: (f64, f64),
    best_start: Option<AttachmentInfo>,
    best_end: Option<AttachmentInfo>,
) -> EdgeDetail {
    EdgeDetail {
        start,
        end,
        start_node: best_start.as_ref().and_then(|info| info.node_id.clone()),
        end_node: best_end.as_ref().and_then(|info| info.node_id.clone()),
        start_edge: best_start
            .as_ref()
            .map(|info| info.edge_name.clone())
            .unwrap_or_else(|| "none".to_string()),
        end_edge: best_end
            .as_ref()
            .map(|info| info.edge_name.clone())
            .unwrap_or_else(|| "none".to_string()),
        start_center_offset: best_start
            .as_ref()
            .map(|info| info.center_offset)
            .unwrap_or(0.0),
        end_center_offset: best_end
            .as_ref()
            .map(|info| info.center_offset)
            .unwrap_or(0.0),
    }
}

/// Extract text element bounding boxes with parent node association
fn extract_text_bounds(doc: &roxmltree::Document, node_bounds: &[NodeBounds]) -> Vec<TextBounds> {
    let mut text_bounds = Vec::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "text" {
            // Get text content from all tspan children or direct text
            let text_content: String = node
                .descendants()
                .filter_map(|n| n.text())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            if text_content.is_empty() {
                continue;
            }

            // Get position from x/y attributes
            let mut x = node
                .attribute("x")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let mut y = node
                .attribute("y")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let (tx, ty) = accumulated_translate(node);
            x += tx;
            y += ty;

            // Estimate text width based on content length and font size
            let font_size = extract_font_size(&node).unwrap_or(16.0);
            let char_width = font_size * 0.6; // Average character width
            let estimated_width = text_content.len() as f64 * char_width;

            // Count lines for height estimation (check for tspan elements)
            let tspan_count = node
                .descendants()
                .filter(|n| n.tag_name().name() == "tspan")
                .count()
                .max(1);
            let estimated_height = tspan_count as f64 * font_size * 1.2;

            // Find parent node if text is inside one
            let parent_node_id = find_parent_node(&node, node_bounds, x, y);

            text_bounds.push(TextBounds {
                x,
                y: y - estimated_height, // Adjust for text baseline
                width: estimated_width,
                height: estimated_height,
                text: text_content,
                parent_node_id,
            });
        }
    }

    text_bounds
}

/// Extract font-size from a text element
fn extract_font_size(node: &roxmltree::Node) -> Option<f64> {
    // Check inline style
    if let Some(style) = node.attribute("style") {
        for part in style.split(';') {
            let kv: Vec<&str> = part.split(':').map(|s| s.trim()).collect();
            if kv.len() == 2 && kv[0] == "font-size" {
                return kv[1].trim_end_matches("px").parse().ok();
            }
        }
    }

    // Check font-size attribute
    node.attribute("font-size")
        .and_then(|s| s.trim_end_matches("px").parse().ok())
}

/// Find if a text element is inside a node bounds
fn find_parent_node(
    text_node: &roxmltree::Node,
    node_bounds: &[NodeBounds],
    text_x: f64,
    text_y: f64,
) -> Option<String> {
    // First check parent groups for node-like classes
    let mut current = text_node.parent();
    while let Some(parent) = current {
        if let Some(class) = parent.attribute("class") {
            if class.contains("node")
                || class.contains("section")
                || class.contains("task")
                || class.contains("event")
            {
                if let Some(id) = parent.attribute("id") {
                    return Some(id.to_string());
                }
                // Use class as fallback ID
                return Some(
                    class
                        .split_whitespace()
                        .next()
                        .unwrap_or("unknown")
                        .to_string(),
                );
            }
        }
        current = parent.parent();
    }

    // Fallback: find geometrically containing node
    for bounds in node_bounds {
        if text_x >= bounds.x
            && text_x <= bounds.x + bounds.width
            && text_y >= bounds.y
            && text_y <= bounds.y + bounds.height
        {
            return Some(bounds.id.clone());
        }
    }

    None
}

#[derive(Debug, PartialEq)]
enum AttachmentType {
    Vertical,   // top or bottom
    Horizontal, // left or right
    None,
}

/// Detailed attachment info for an edge endpoint
struct AttachmentInfo {
    attach_type: AttachmentType,
    edge_name: String,       // "top", "bottom", "left", "right", "none"
    node_id: Option<String>, // ID of the node this attaches to
    center_offset: f64,      // Distance from center of that edge (0 = centered)
    distance: f64,           // Distance from the edge
}

/// Classify how a point attaches to a node bounds with detailed info
/// Returns the CLOSEST matching edge within tolerance, not the first match
fn classify_attachment_detailed(point: (f64, f64), bounds: &NodeBounds) -> AttachmentInfo {
    let (px, py) = point;
    let tolerance = 25.0; // Increased tolerance to account for marker offsets

    let left = bounds.x;
    let right = bounds.x + bounds.width;
    let top = bounds.y;
    let bottom = bounds.y + bounds.height;
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;

    // Check proximity to each edge
    let dist_top = (py - top).abs();
    let dist_bottom = (py - bottom).abs();
    let dist_left = (px - left).abs();
    let dist_right = (px - right).abs();

    let within_x = px >= left - tolerance && px <= right + tolerance;
    let within_y = py >= top - tolerance && py <= bottom + tolerance;

    // Collect all matching edges within tolerance
    let mut candidates = Vec::new();

    if dist_top < tolerance && within_x {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Vertical,
            edge_name: "top".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: px - center_x,
            distance: dist_top,
        });
    }
    if dist_bottom < tolerance && within_x {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Vertical,
            edge_name: "bottom".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: px - center_x,
            distance: dist_bottom,
        });
    }
    if dist_left < tolerance && within_y {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Horizontal,
            edge_name: "left".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: py - center_y,
            distance: dist_left,
        });
    }
    if dist_right < tolerance && within_y {
        candidates.push(AttachmentInfo {
            attach_type: AttachmentType::Horizontal,
            edge_name: "right".to_string(),
            node_id: Some(bounds.id.clone()),
            center_offset: py - center_y,
            distance: dist_right,
        });
    }

    // Return the candidate with the smallest distance
    candidates
        .into_iter()
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| AttachmentInfo {
            attach_type: AttachmentType::None,
            edge_name: "none".to_string(),
            node_id: None,
            center_offset: 0.0,
            distance: f64::MAX,
        })
}

/// Classify how a point attaches to a node bounds (legacy simple version)
fn classify_attachment(point: (f64, f64), bounds: &NodeBounds) -> (AttachmentType, f64) {
    let info = classify_attachment_detailed(point, bounds);
    (info.attach_type, info.distance)
}

/// Parse transform="translate(x, y)" or "translate(x,y)"
fn parse_translate(transform: &str) -> Option<(f64, f64)> {
    // Look for translate(x, y) pattern
    if let Some(start) = transform.find("translate(") {
        let rest = &transform[start + 10..];
        if let Some(end) = rest.find(')') {
            let coords = &rest[..end];
            // Split by comma or space, filter empty parts
            let parts: Vec<&str> = coords.split([',', ' ']).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 {
                let x = parts[0].trim().parse::<f64>().ok()?;
                let y = parts[1].trim().parse::<f64>().ok()?;
                return Some((x, y));
            }
        }
    }
    None
}

fn accumulated_translate(node: roxmltree::Node<'_, '_>) -> (f64, f64) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut current = Some(node);

    while let Some(current_node) = current {
        if let Some(transform) = current_node.attribute("transform") {
            if let Some((tx, ty)) = parse_translate(transform) {
                x += tx;
                y += ty;
            }
        }
        current = current_node.parent();
    }

    (x, y)
}

/// Parse rectangular path dimensions from mermaid's path d attribute
/// e.g., "M-93.828125 -85.5 L93.828125 -85.5 L93.828125 85.5 L-93.828125 85.5"
/// Returns (half_width, half_height)
fn parse_rect_path_dimensions(d: &str) -> Option<(f64, f64)> {
    // Mermaid paths start with M followed by negative half-width and half-height
    // e.g., M-93.828125 -85.5 means center is at (0,0), box is from -93.8 to +93.8
    let parts: Vec<&str> = d.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Parse first M command to get the top-left corner (negative values)
    let first = parts.first()?;
    if let Some(coords) = first.strip_prefix('M') {
        // Handle "M-93.828125" followed by "-85.5" or "M-93,-85"
        let x = if coords.is_empty() {
            parts.get(1)?.parse::<f64>().ok()?
        } else {
            coords.parse::<f64>().ok()?
        };

        // Get y value (might be second element or after comma)
        let y = if coords.is_empty() || !coords.contains(',') {
            let y_str = if coords.is_empty() {
                parts.get(2)?
            } else {
                parts.get(1)?
            };
            y_str.parse::<f64>().ok()?
        } else {
            let comma_idx = coords.find(',')?;
            coords[comma_idx + 1..].parse::<f64>().ok()?
        };

        // Only valid if both coordinates are negative (mermaid rect style)
        // This distinguishes from timeline paths that start at M0 Y
        if x < 0.0 && y < 0.0 {
            // Return absolute values as half-dimensions
            return Some((x.abs(), y.abs()));
        }
    }

    None
}

/// Parse timeline path dimensions from paths like "M0 63 v-58 q0,-5 5,-5 h210 q5,0 5,5 v63 H0 Z"
/// Returns (width, height) based on the path's bounding box
fn parse_timeline_path_dimensions(d: &str) -> Option<(f64, f64)> {
    // Timeline paths start at M0 Y and use relative commands (v, h, q)
    // We need to find the maximum extents
    let normalized = normalize_path_commands(d);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    let mut bounds = TimelinePathBounds::default();

    while bounds.index < parts.len() {
        bounds.apply_part(&parts)?;
        bounds.index += 1;
    }

    bounds.dimensions()
}

#[derive(Debug, Default)]
struct TimelinePathBounds {
    x: f64,
    y: f64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    index: usize,
}

impl TimelinePathBounds {
    fn apply_part(&mut self, parts: &[&str]) -> Option<()> {
        let part = parts[self.index];

        if part == "M" || part.starts_with('M') {
            self.apply_move(part, parts)?;
        } else if part == "v" || part.starts_with('v') {
            let dy = self.command_value(part, parts, 'v')?;
            self.move_y_relative(dy);
        } else if part == "V" || part.starts_with('V') {
            let y = self.command_value(part, parts, 'V')?;
            self.move_y_absolute(y);
        } else if part == "h" || part.starts_with('h') {
            let dx = self.command_value(part, parts, 'h')?;
            self.move_x_relative(dx);
        } else if part == "H" || part.starts_with('H') {
            let x = self.command_value(part, parts, 'H')?;
            self.move_x_absolute(x);
        } else if part == "q" || part.starts_with('q') {
            self.apply_relative_quadratic(part, parts)?;
        }

        Some(())
    }

    fn apply_move(&mut self, part: &str, parts: &[&str]) -> Option<()> {
        let (x, y) = if part == "M" {
            self.index += 1;
            let mx = parts.get(self.index)?.parse::<f64>().ok()?;
            self.index += 1;
            let my = parts.get(self.index)?.parse::<f64>().ok()?;
            (mx, my)
        } else if let Some((px, py)) = parse_inline_coords(&part[1..]) {
            (px, py)
        } else {
            let mx = part[1..].parse::<f64>().ok()?;
            self.index += 1;
            let my = parts.get(self.index)?.parse::<f64>().ok()?;
            (mx, my)
        };

        self.x = x;
        self.y = y;
        self.update_bounds();
        Some(())
    }

    fn command_value(&mut self, part: &str, parts: &[&str], command: char) -> Option<f64> {
        if part == command.to_string() {
            self.index += 1;
            parts.get(self.index)?.parse::<f64>().ok()
        } else {
            part[1..].parse::<f64>().ok()
        }
    }

    fn apply_relative_quadratic(&mut self, part: &str, parts: &[&str]) -> Option<()> {
        if part == "q" {
            self.index += 1; // cx
            self.index += 1; // cy
            self.index += 1; // ex
            let ex = parts
                .get(self.index - 1)?
                .trim_matches(',')
                .parse::<f64>()
                .ok()?;
            let ey = parts
                .get(self.index)?
                .trim_matches(',')
                .parse::<f64>()
                .ok()?;
            self.move_relative(ex, ey);
        } else {
            self.index += 1;
            if let Some((ex, ey)) = parts
                .get(self.index)
                .and_then(|part| parse_inline_coords(part))
            {
                self.move_relative(ex, ey);
            }
        }

        Some(())
    }

    fn move_x_relative(&mut self, dx: f64) {
        self.x += dx;
        self.update_bounds();
    }

    fn move_x_absolute(&mut self, x: f64) {
        self.x = x;
        self.update_bounds();
    }

    fn move_y_relative(&mut self, dy: f64) {
        self.y += dy;
        self.update_bounds();
    }

    fn move_y_absolute(&mut self, y: f64) {
        self.y = y;
        self.update_bounds();
    }

    fn move_relative(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        self.update_bounds();
    }

    fn update_bounds(&mut self) {
        self.min_x = self.min_x.min(self.x);
        self.max_x = self.max_x.max(self.x);
        self.min_y = self.min_y.min(self.y);
        self.max_y = self.max_y.max(self.y);
    }

    fn dimensions(&self) -> Option<(f64, f64)> {
        let width = self.max_x - self.min_x;
        let height = self.max_y - self.min_y;

        if width > 0.0 && height > 0.0 {
            Some((width, height))
        } else {
            None
        }
    }
}

/// Parse path with initial direction: returns (start, second_point, end)
/// The second_point is used to determine the initial tangent direction of curved paths.
/// For paths like "M122,451 L122,459 C...", the second point is (122,459) which shows
/// the edge starts going DOWN even if the overall direction is diagonal.
#[allow(clippy::type_complexity)]
fn parse_path_with_directions(d: &str) -> Option<((f64, f64), Option<(f64, f64)>, (f64, f64))> {
    let normalized = normalize_path_commands(d);
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut cursor = PathDirectionCursor::default();
    while cursor.index < parts.len() {
        if cursor.apply_part(&parts)? {
            cursor.index += 1;
        }
    }

    match (cursor.start, cursor.end) {
        (Some(s), Some(e)) => Some((s, cursor.second_point, e)),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct PathDirectionCursor {
    start: Option<(f64, f64)>,
    second_point: Option<(f64, f64)>,
    end: Option<(f64, f64)>,
    point_count: usize,
    index: usize,
}

impl PathDirectionCursor {
    fn apply_part(&mut self, parts: &[&str]) -> Option<bool> {
        let part = parts[self.index];

        if part == "M" || part.starts_with('M') {
            self.apply_move(part, parts)
        } else if part == "L" || part.starts_with('L') {
            self.apply_line(part, parts)
        } else if part == "C" || part.starts_with('C') {
            self.apply_cubic(part, parts)
        } else if part == "Q" || part.starts_with('Q') {
            self.apply_quadratic(part, parts)
        } else if let Some(point) = parse_inline_coords(part) {
            self.record_line_point(point);
            Some(true)
        } else {
            Some(true)
        }
    }

    fn apply_move(&mut self, part: &str, parts: &[&str]) -> Option<bool> {
        let point = if part == "M" {
            self.index += 1;
            let point = parse_coord_pair(parts, &mut self.index)?;
            self.record_move_point(point);
            return Some(false);
        } else {
            parse_inline_coords(&part[1..])?
        };

        self.record_move_point(point);
        Some(true)
    }

    fn apply_line(&mut self, part: &str, parts: &[&str]) -> Option<bool> {
        let point = if part == "L" {
            self.index += 1;
            let point = parse_coord_pair(parts, &mut self.index)?;
            self.record_line_point(point);
            return Some(false);
        } else {
            parse_inline_coords(&part[1..])?
        };

        self.record_line_point(point);
        Some(true)
    }

    fn apply_cubic(&mut self, part: &str, parts: &[&str]) -> Option<bool> {
        if part == "C" {
            self.index += 1;
            let first_control = parse_coord_pair(parts, &mut self.index)?;
            self.record_curve_control(first_control);
            parse_coord_pair(parts, &mut self.index)?;
            let endpoint = parse_coord_pair(parts, &mut self.index)?;
            self.record_curve_endpoint(endpoint);
            return Some(false);
        }

        let coords = parse_path_command_numbers(&part[1..]);
        if coords.len() >= 6 {
            self.record_curve_control((coords[0], coords[1]));
            self.record_curve_endpoint((coords[4], coords[5]));
        }
        Some(true)
    }

    fn apply_quadratic(&mut self, part: &str, parts: &[&str]) -> Option<bool> {
        if part == "Q" {
            self.index += 1;
            let control = parse_coord_pair(parts, &mut self.index)?;
            self.record_curve_control(control);
            let endpoint = parse_coord_pair(parts, &mut self.index)?;
            self.record_curve_endpoint(endpoint);
            return Some(false);
        }

        let coords = parse_path_command_numbers(&part[1..]);
        if coords.len() >= 4 {
            self.record_curve_control((coords[0], coords[1]));
            self.record_curve_endpoint((coords[2], coords[3]));
        }
        Some(true)
    }

    fn record_move_point(&mut self, point: (f64, f64)) {
        if self.start.is_none() {
            self.start = Some(point);
            self.point_count = 1;
        }
        self.end = Some(point);
    }

    fn record_line_point(&mut self, point: (f64, f64)) {
        self.point_count += 1;
        self.record_second_point_if_needed(point);
        self.end = Some(point);
    }

    fn record_curve_control(&mut self, point: (f64, f64)) {
        if self.point_count == 1 && self.second_point.is_none() {
            self.second_point = Some(point);
        }
    }

    fn record_curve_endpoint(&mut self, point: (f64, f64)) {
        self.point_count += 1;
        self.end = Some(point);
    }

    fn record_second_point_if_needed(&mut self, point: (f64, f64)) {
        if self.point_count == 2 && self.second_point.is_none() {
            self.second_point = Some(point);
        }
    }
}

fn parse_path_command_numbers(coords: &str) -> Vec<f64> {
    coords
        .split([',', ' '])
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Normalize SVG path commands by inserting spaces before command letters.
/// This handles compact mermaid paths like "M122,179L122,280C..." by converting to
/// "M122,179 L122,280 C..."
fn normalize_path_commands(d: &str) -> String {
    let mut result = String::with_capacity(d.len() * 2);

    for c in d.chars() {
        // Insert space before command letters (except for first character and after another space)
        if matches!(
            c,
            'M' | 'L'
                | 'C'
                | 'Q'
                | 'A'
                | 'H'
                | 'V'
                | 'Z'
                | 'm'
                | 'l'
                | 'c'
                | 'q'
                | 'a'
                | 'h'
                | 'v'
                | 'z'
        ) {
            // Add space before command letter if not at start and previous char isn't space
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            result.push(c);
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_coord_pair(parts: &[&str], i: &mut usize) -> Option<(f64, f64)> {
    if *i >= parts.len() {
        return None;
    }

    let part = parts[*i];

    // Try to parse as "x,y" or "x y"
    if let Some((x, y)) = parse_inline_coords(part) {
        *i += 1; // Advance past this part
        return Some((x, y));
    }

    // Try separate x and y values
    // Strip leading/trailing commas that may appear in paths like "C x y, x y, x y"
    let x: f64 = part.trim_matches(',').parse().ok()?;
    *i += 1;
    if *i >= parts.len() {
        return None;
    }
    let y: f64 = parts[*i].trim_matches(',').parse().ok()?;
    *i += 1; // Advance past y value
    Some((x, y))
}

fn parse_inline_coords(s: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 2 {
        let x: f64 = parts[0].parse().ok()?;
        let y: f64 = parts[1].parse().ok()?;
        return Some((x, y));
    }
    None
}

/// Analyze font styles (size, weight) on text elements
fn analyze_fonts(doc: &roxmltree::Document) -> FontAnalysis {
    let mut analysis = FontAnalysis::default();

    // Extract CSS font rules if present (for eval feature)
    #[cfg(feature = "eval")]
    let css_fonts = extract_css_font_styles(doc);
    #[cfg(not(feature = "eval"))]
    let css_fonts: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "text" {
            analysis.text_count += 1;

            // Get context from class attribute
            let class = node.attribute("class").unwrap_or("").to_string();
            let context = if class.is_empty() {
                "text".to_string()
            } else {
                class.clone()
            };

            // Check inline font-size attribute
            if let Some(size) = node.attribute("font-size") {
                analysis.font_sizes.push(FontStyle {
                    context: context.clone(),
                    value: size.to_string(),
                });
            } else {
                // Check CSS rules for matching class
                for css_class in class.split_whitespace() {
                    if let Some((Some(s), _)) = css_fonts.get(css_class) {
                        analysis.font_sizes.push(FontStyle {
                            context: context.clone(),
                            value: s.clone(),
                        });
                        break;
                    }
                }
            }

            // Check inline font-weight attribute
            if let Some(weight) = node.attribute("font-weight") {
                analysis.font_weights.push(FontStyle {
                    context: context.clone(),
                    value: weight.to_string(),
                });
            } else {
                // Check CSS rules for matching class
                for css_class in class.split_whitespace() {
                    if let Some((_, Some(w))) = css_fonts.get(css_class) {
                        analysis.font_weights.push(FontStyle {
                            context: context.clone(),
                            value: w.clone(),
                        });
                        break;
                    }
                }
            }

            // Also check inline style attribute
            if let Some(style) = node.attribute("style") {
                if let Some(size) = extract_style_property(style, "font-size") {
                    analysis.font_sizes.push(FontStyle {
                        context: context.clone(),
                        value: size,
                    });
                }
                if let Some(weight) = extract_style_property(style, "font-weight") {
                    analysis.font_weights.push(FontStyle {
                        context,
                        value: weight,
                    });
                }
            }
        }
    }

    analysis
}

/// Analyze colors (fill and stroke) used in the SVG
fn analyze_colors(doc: &roxmltree::Document) -> ColorAnalysis {
    let mut colors = SvgColorUsage::default();

    // First, extract colors from CSS <style> elements
    // This ensures we capture colors applied via CSS rules like .node rect { fill: #ECECFF }
    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                extract_css_colors(css_text, &mut colors.fill_colors, &mut colors.stroke_colors);
            }
        }
    }

    for node in doc.descendants() {
        colors.collect_node(&node);
    }

    let mut fill_vec: Vec<String> = colors.fill_colors.into_iter().collect();
    let mut stroke_vec: Vec<String> = colors.stroke_colors.into_iter().collect();
    fill_vec.sort();
    stroke_vec.sort();

    // Detect text visibility issues (CSS fill override)
    let text_visibility_issues = detect_text_visibility_issues(doc);

    ColorAnalysis {
        fill_colors: fill_vec,
        stroke_colors: stroke_vec,
        fill_count: colors.fill_count,
        stroke_count: colors.stroke_count,
        text_visibility_issues,
    }
}

#[derive(Debug, Default)]
struct SvgColorUsage {
    fill_colors: std::collections::HashSet<String>,
    stroke_colors: std::collections::HashSet<String>,
    fill_count: usize,
    stroke_count: usize,
}

impl SvgColorUsage {
    fn collect_node(&mut self, node: &roxmltree::Node<'_, '_>) {
        let tag = node.tag_name().name();
        if is_non_rendered_color_tag(tag) {
            return;
        }

        let is_shape = is_color_shape_tag(tag);
        self.collect_color_attr(node.attribute("fill"), is_shape, true);
        self.collect_color_attr(node.attribute("stroke"), is_shape, false);

        if let Some(style) = node.attribute("style") {
            self.collect_style_colors(style, is_shape);
        }
    }

    fn collect_style_colors(&mut self, style: &str, is_shape: bool) {
        self.collect_color_attr(
            extract_style_property(style, "fill").as_deref(),
            is_shape,
            true,
        );
        self.collect_color_attr(
            extract_style_property(style, "stroke").as_deref(),
            is_shape,
            false,
        );
    }

    fn collect_color_attr(&mut self, value: Option<&str>, is_shape: bool, is_fill: bool) {
        let Some(color) = value.filter(|color| *color != "none" && !color.is_empty()) else {
            return;
        };

        if is_fill {
            self.fill_colors.insert(normalize_color(color));
            self.fill_count += usize::from(is_shape);
        } else {
            self.stroke_colors.insert(normalize_color(color));
            self.stroke_count += usize::from(is_shape);
        }
    }
}

fn is_non_rendered_color_tag(tag: &str) -> bool {
    matches!(tag, "defs" | "marker" | "clipPath" | "mask")
}

fn is_color_shape_tag(tag: &str) -> bool {
    matches!(
        tag,
        "rect" | "circle" | "ellipse" | "polygon" | "path" | "line" | "polyline"
    )
}

/// Extract fill and stroke colors from CSS text
fn extract_css_colors(
    css_text: &str,
    fill_colors: &mut std::collections::HashSet<String>,
    stroke_colors: &mut std::collections::HashSet<String>,
) {
    // Parse CSS rules to extract fill and stroke colors
    // Format: selector { property: value; ... }
    for rule in css_text.split('}') {
        let rule = rule.trim();
        if rule.is_empty() {
            continue;
        }

        if let Some(brace_pos) = rule.find('{') {
            let properties = rule[brace_pos + 1..].trim();

            for prop in properties.split(';') {
                let prop = prop.trim();
                if prop.is_empty() {
                    continue;
                }

                if let Some(colon_pos) = prop.find(':') {
                    let name = prop[..colon_pos].trim().to_lowercase();
                    let value = prop[colon_pos + 1..].trim();

                    // Skip values like "none", "inherit", "transparent", CSS variables, rgba
                    if value == "none"
                        || value == "inherit"
                        || value == "transparent"
                        || value.starts_with("var(")
                        || value.starts_with("url(")
                    {
                        continue;
                    }

                    // Extract color value (handle "!important" suffix)
                    let color_value = value.split('!').next().unwrap_or(value).trim();
                    if color_value.is_empty() {
                        continue;
                    }

                    if name == "fill" {
                        fill_colors.insert(normalize_color(color_value));
                    } else if name == "stroke" {
                        stroke_colors.insert(normalize_color(color_value));
                    }
                }
            }
        }
    }
}

/// Detect text elements where CSS fill rules may override inline fill attributes
/// This can cause text to become invisible or hard to read against its background
fn detect_text_visibility_issues(doc: &roxmltree::Document) -> Vec<TextVisibilityIssue> {
    use std::collections::HashMap;

    let mut issues = Vec::new();

    // Step 1: Parse CSS from <style> elements to build class -> fill map
    let mut css_fill_rules: HashMap<String, String> = HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                // Parse simple CSS rules like ".class-name { fill: #color; }"
                for rule in css_text.split('}') {
                    let rule = rule.trim();
                    if rule.is_empty() {
                        continue;
                    }

                    // Split into selector and properties
                    if let Some(brace_pos) = rule.find('{') {
                        let selector = rule[..brace_pos].trim();
                        let properties = rule[brace_pos + 1..].trim();

                        // Extract fill property if present
                        for prop in properties.split(';') {
                            let prop = prop.trim();
                            if let Some(fill_value) = prop.strip_prefix("fill:") {
                                let fill_value = fill_value.trim().to_lowercase();

                                // Handle multiple selectors (e.g., ".class1, .class2")
                                for sel in selector.split(',') {
                                    let sel = sel.trim();
                                    // Extract class name from selector (e.g., ".section-type-0" -> "section-type-0")
                                    if let Some(class_name) = sel.strip_prefix('.') {
                                        // Handle compound selectors by taking the last class
                                        let class_name = class_name
                                            .split_whitespace()
                                            .next()
                                            .unwrap_or(class_name);
                                        css_fill_rules
                                            .insert(class_name.to_string(), fill_value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 2: Find text elements with classes that have CSS fill rules
    for node in doc.descendants() {
        if node.tag_name().name() != "text" {
            continue;
        }

        let class_attr = node.attribute("class").unwrap_or("");
        let inline_fill = node.attribute("fill").map(String::from);
        let text_content = get_text_content(&node);

        // Check each class on the text element
        for class_name in class_attr.split_whitespace() {
            if let Some(css_fill) = css_fill_rules.get(class_name) {
                // This text has a class with a CSS fill rule
                // Check if the inline fill differs from CSS (potential override issue)
                if let Some(ref inline) = inline_fill {
                    let inline_normalized = normalize_color(inline);
                    if inline_normalized != *css_fill {
                        // CSS fill differs from inline fill - this is a potential issue
                        // because CSS class rules override SVG presentation attributes

                        // Try to find background color from sibling rect
                        let background_fill = find_sibling_rect_fill(&node);

                        issues.push(TextVisibilityIssue {
                            text: text_content.clone(),
                            css_class: class_name.to_string(),
                            css_fill: css_fill.clone(),
                            inline_fill: Some(inline_normalized),
                            background_fill,
                        });
                        break; // Only report once per text element
                    }
                }
            }
        }
    }

    issues
}

/// Find the fill color of a sibling rect element (likely the background)
fn find_sibling_rect_fill(text_node: &roxmltree::Node) -> Option<String> {
    // Look for a sibling rect in the same parent group
    if let Some(parent) = text_node.parent() {
        for sibling in parent.children() {
            if sibling.tag_name().name() == "rect" {
                if let Some(fill) = sibling.attribute("fill") {
                    return Some(normalize_color(fill));
                }
            }
        }
    }
    None
}

/// Get text content from a text element (including tspan children)
fn get_text_content(node: &roxmltree::Node) -> String {
    let mut content = String::new();

    // Get direct text content
    if let Some(text) = node.text() {
        content.push_str(text);
    }

    // Get text from tspan children
    for child in node.children() {
        if child.tag_name().name() == "tspan" {
            if let Some(text) = child.text() {
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(text);
            }
        }
    }

    content.trim().to_string()
}

/// Normalize a color string for comparison
/// Converts to lowercase and handles common formats
fn normalize_color(color: &str) -> String {
    let color = color.trim().to_lowercase();

    // Handle rgb/rgba by converting to canonical form
    if color.starts_with("rgb") {
        // Already in rgb format, just normalize spacing
        color
            .replace(", ", ",")
            .replace(" ,", ",")
            .replace("( ", "(")
            .replace(" )", ")")
    } else if color.starts_with("hsl") {
        // HSL format - normalize spacing
        color
            .replace(", ", ",")
            .replace(" ,", ",")
            .replace("( ", "(")
            .replace(" )", ")")
    } else {
        color
    }
}

/// Extract a property value from an inline style string
fn extract_style_property(style: &str, property: &str) -> Option<String> {
    for part in style.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix(property) {
            if let Some(v) = value.strip_prefix(':') {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// Extract font-size and font-weight from CSS style blocks
#[cfg(feature = "eval")]
fn extract_css_font_styles(
    doc: &roxmltree::Document,
) -> std::collections::HashMap<String, (Option<String>, Option<String>)> {
    use simplecss::StyleSheet;
    let mut css_fonts = std::collections::HashMap::new();

    for node in doc.descendants() {
        if node.tag_name().name() == "style" {
            if let Some(css_text) = node.text() {
                let stylesheet = StyleSheet::parse(css_text);
                for rule in stylesheet.rules {
                    let mut font_size: Option<String> = None;
                    let mut font_weight: Option<String> = None;

                    for decl in &rule.declarations {
                        if decl.name == "font-size" {
                            font_size = Some(decl.value.trim().to_string());
                        } else if decl.name == "font-weight" {
                            font_weight = Some(decl.value.trim().to_string());
                        }
                    }

                    // Associate with each selector in the rule
                    if font_size.is_some() || font_weight.is_some() {
                        let selector_str = rule.selector.to_string();
                        for selector in selector_str.split(',') {
                            let sel = selector.trim();
                            // Extract class name from selector (e.g., ".entity-name" -> "entity-name")
                            if let Some(class_name) = sel.strip_prefix('.') {
                                let class_name = class_name.split_whitespace().next().unwrap_or("");
                                css_fonts.insert(
                                    class_name.to_string(),
                                    (font_size.clone(), font_weight.clone()),
                                );
                            }
                            // Also handle ID selectors (e.g., "#my-svg" -> "root")
                            // and element selectors (e.g., "svg" -> "root")
                            // These are typically used for default/inherited font sizes
                            else if sel.starts_with('#') || sel == "svg" || sel.ends_with(" svg")
                            {
                                css_fonts.insert(
                                    "root".to_string(),
                                    (font_size.clone(), font_weight.clone()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    css_fonts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_labels_combines_tspans() {
        // Mermaid.js splits multi-word text into separate tspan elements
        let mermaid_style_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <text>
                <tspan>Main</tspan>
                <tspan> Flow</tspan>
            </text>
        </svg>"#;

        let structure = SvgStructure::from_svg(mermaid_style_svg).unwrap();

        // Should extract "Main Flow" as a single label, not ["Main", " Flow"]
        assert!(
            structure.labels.contains(&"Main Flow".to_string()),
            "Should combine tspans into single label. Got: {:?}",
            structure.labels
        );
        assert!(
            !structure.labels.iter().any(|l| l == "Main" || l == " Flow"),
            "Should not have separate tspan fragments. Got: {:?}",
            structure.labels
        );
    }

    #[test]
    fn test_extract_multiline_tspans_combines_all() {
        // Multi-line text uses dy attribute to position lines
        let multiline_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <text x="10" y="20">
                <tspan x="10" y="20">Line one</tspan>
                <tspan x="10" dy="1.2em">Line two</tspan>
                <tspan x="10" dy="1.2em">Line three</tspan>
            </text>
        </svg>"#;

        let structure = SvgStructure::from_svg(multiline_svg).unwrap();

        // Should combine all tspans into a single label (matching HTML <p> behavior)
        assert!(
            structure
                .labels
                .contains(&"Line one Line two Line three".to_string()),
            "Should combine all tspans into single label. Got: {:?}",
            structure.labels
        );
    }

    #[test]
    fn test_extract_labels_p_with_br_collects_full_text() {
        // Mermaid.js mindmap uses foreignObject with <p> containing <br/> for line breaks
        // The eval should extract the full text, not just the first text child
        let mermaid_mindmap_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 200 100">
            <g class="node">
                <foreignObject width="120" height="48">
                    <div xmlns="http://www.w3.org/1999/xhtml">
                        <span class="nodeLabel"><p>On effectiveness<br/>and features</p></span>
                    </div>
                </foreignObject>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(mermaid_mindmap_svg).unwrap();

        // Should extract "On effectiveness and features" as a single label
        assert!(
            structure
                .labels
                .contains(&"On effectiveness and features".to_string()),
            "Should extract full text from <p> with <br/> tags. Got: {:?}",
            structure.labels
        );
        // Should NOT have partial text
        assert!(
            !structure.labels.iter().any(|l| l == "On effectiveness"),
            "Should not extract partial text before <br/>. Got: {:?}",
            structure.labels
        );
    }

    #[test]
    fn test_count_visible_rects_only() {
        // Mermaid.js style SVG with helper rects (empty rects inside labels)
        let mermaid_style_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="nodes">
                <g class="node">
                    <rect class="label-container" x="10" y="10" width="80" height="40"/>
                    <g class="label">
                        <rect></rect>
                        <text>Label</text>
                    </g>
                </g>
            </g>
            <g class="edgeLabels">
                <g><rect class="background" style="stroke: none"></rect></g>
            </g>
        </svg>"#;

        // Our clean SVG with just the visible rect
        let clean_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="nodes">
                <g class="node">
                    <rect x="10" y="10" width="80" height="40"/>
                    <text>Label</text>
                </g>
            </g>
        </svg>"#;

        let mermaid_structure = SvgStructure::from_svg(mermaid_style_svg).unwrap();
        let clean_structure = SvgStructure::from_svg(clean_svg).unwrap();

        // Both should report the same number of VISIBLE rects (1)
        // Currently this will fail because we count all rects
        assert_eq!(
            mermaid_structure.shapes.rect, clean_structure.shapes.rect,
            "Should count only visible rects, not helper elements. Mermaid has {} rects, clean has {}",
            mermaid_structure.shapes.rect, clean_structure.shapes.rect
        );
    }

    #[test]
    fn test_architecture_counts_nodes_and_edges() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <g class="architecture-edges">
                <g><path class="edge" d="M 0 0 L 10 10"/></g>
            </g>
            <g class="architecture-services">
                <g class="architecture-service"></g>
                <g class="architecture-junction"></g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        assert_eq!(structure.node_count, 2);
        assert_eq!(structure.edge_count, 1);
    }

    #[test]
    fn test_architecture_node_bounds_extraction() {
        // Architecture services use nested <svg> icons within a translated <g>
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400">
            <g class="architecture-edges">
                <g><path class="edge" d="M 40,0 L 160,0"/></g>
            </g>
            <g class="architecture-services">
                <g class="architecture-service" transform="translate(-40, -40)" id="service-gateway">
                    <g transform="translate(40, 80)"><text>Gateway</text></g>
                    <g><g><svg xmlns="http://www.w3.org/2000/svg" width="80" height="80" viewBox="0 0 80 80"><rect width="80" height="80"/></svg></g></g>
                </g>
                <g class="architecture-service" transform="translate(160, -40)" id="service-server">
                    <g transform="translate(40, 80)"><text>Server</text></g>
                    <g><g><svg xmlns="http://www.w3.org/2000/svg" width="80" height="80" viewBox="0 0 80 80"><rect width="80" height="80"/></svg></g></g>
                </g>
                <g class="architecture-junction" transform="translate(360, 160)">
                    <g><rect x="0" y="0" width="80" height="80" fill-opacity="0" id="node-hub"/></g>
                </g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        let bounds = &structure.edge_geometry.node_bounds;

        // Should extract bounds for all 3 architecture nodes
        assert!(
            bounds.len() >= 3,
            "Expected at least 3 node bounds for architecture nodes, got {}",
            bounds.len()
        );

        // gateway at translate(-40, -40) with 80x80 icon
        let gateway = bounds.iter().find(|b| b.id.contains("gateway"));
        assert!(gateway.is_some(), "Should find gateway bounds");
        let gw = gateway.unwrap();
        assert_eq!(gw.x, -40.0);
        assert_eq!(gw.y, -40.0);
        assert_eq!(gw.width, 80.0);
        assert_eq!(gw.height, 80.0);

        // server at translate(160, -40) with 80x80 icon
        let server = bounds.iter().find(|b| b.id.contains("server"));
        assert!(server.is_some(), "Should find server bounds");
        let sv = server.unwrap();
        assert_eq!(sv.x, 160.0);
        assert_eq!(sv.y, -40.0);

        // Junction node should also be extracted
        let junction_bounds: Vec<_> = bounds
            .iter()
            .filter(|b| b.x == 360.0 && b.y == 160.0)
            .collect();
        assert!(
            !junction_bounds.is_empty(),
            "Should find junction bounds at (360, 160)"
        );
    }

    #[test]
    fn test_nested_transforms_apply_to_flowchart_geometry() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 500">
            <g transform="translate(100, 200)">
                <g class="nodes">
                    <g class="node default" id="flowchart-A-0" transform="translate(50, 60)">
                        <rect class="basic label-container" x="-20" y="-10" width="40" height="20"/>
                    </g>
                    <g class="node default" id="flowchart-B-1" transform="translate(150, 60)">
                        <rect class="basic label-container" x="-20" y="-10" width="40" height="20"/>
                    </g>
                </g>
                <g class="edgePaths">
                    <path class="flowchart-link" data-edge="true" d="M70,60L130,60"/>
                </g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        let bounds = &structure.edge_geometry.node_bounds;
        let first = bounds
            .iter()
            .find(|bounds| bounds.id == "flowchart-A-0")
            .expect("node A bounds should be extracted");

        assert_eq!(first.x, 130.0);
        assert_eq!(first.y, 250.0);
        assert_eq!(
            structure.edge_geometry.edge_endpoints,
            vec![(170.0, 260.0, 230.0, 260.0)]
        );
    }

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect x="10" y="10" width="80" height="40"/>
            <text x="50" y="35">Hello</text>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();
        assert_eq!(structure.width, 200.0);
        assert_eq!(structure.height, 100.0);
        assert_eq!(structure.shapes.rect, 1);
        assert!(structure.labels.contains(&"Hello".to_string()));
    }

    #[test]
    fn test_compare_identical() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
            <rect class="node" x="10" y="10" width="80" height="40"/>
            <text>Label</text>
        </svg>"#;

        let s1 = SvgStructure::from_svg(svg).unwrap();
        let s2 = SvgStructure::from_svg(svg).unwrap();

        assert_eq!(s1, s2);
    }

    #[test]
    fn test_compare_different_dimensions() {
        let svg1 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"></svg>"#;
        let svg2 = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200"></svg>"#;

        let s1 = SvgStructure::from_svg(svg1).unwrap();
        let s2 = SvgStructure::from_svg(svg2).unwrap();

        assert_ne!(s1.width, s2.width);
        assert_ne!(s1.height, s2.height);
    }

    #[test]
    fn test_mermaid_er_data_edge_detection() {
        // Simplified mermaid ER diagram SVG with data-edge attribute
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 600">
            <g class="edgePaths">
                <path d="M122,179 L122,280"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"
                      marker-start="url(#er-onlyOneStart)"
                      marker-end="url(#er-zeroOrMoreEnd)"/>
                <path d="M122,451 L237,564"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"/>
                <path d="M463,451 L348,564"
                      class="edge-thickness-normal edge-pattern-solid relationshipLine"
                      data-edge="true"/>
            </g>
            <g class="nodes">
                <g class="node default" id="entity-CUSTOMER-0" transform="translate(122, 93.5)">
                    <path d="M-94 -85.5 L94 -85.5 L94 85.5 L-94 85.5"/>
                </g>
                <g class="node default" id="entity-ORDER-1" transform="translate(122, 365.5)">
                    <path d="M-114 -85.5 L114 -85.5 L114 85.5 L-114 85.5"/>
                </g>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();

        // Should detect 3 edges via data-edge attribute
        assert_eq!(
            structure.edge_count, 3,
            "Expected 3 edges from data-edge attribute, got {}",
            structure.edge_count
        );

        // Should detect nodes
        assert!(
            structure.node_count >= 2,
            "Expected at least 2 nodes, got {}",
            structure.node_count
        );

        // Should have edge geometry details
        assert_eq!(
            structure.edge_geometry.edge_endpoints.len(),
            3,
            "Expected 3 edge endpoints"
        );
    }

    #[test]
    fn test_mermaid_minified_data_edge_detection() {
        // Minified mermaid SVG (all on one line) - this is what we get from mermaid.js
        let svg = r#"<svg id="my-svg" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 600"><g><g class="edgePaths"><path d="M122,179L122,280" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true" marker-start="url(#er-onlyOneStart)" marker-end="url(#er-zeroOrMoreEnd)"/><path d="M122,451L237,564" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true"/><path d="M463,451L348,564" class="edge-thickness-normal edge-pattern-solid relationshipLine" data-edge="true"/></g></g></svg>"#;

        let structure = SvgStructure::from_svg(svg).unwrap();

        // Should detect 3 edges via data-edge attribute
        assert_eq!(
            structure.edge_count, 3,
            "Expected 3 edges from minified SVG data-edge attribute, got {}",
            structure.edge_count
        );
    }

    #[test]
    fn test_real_mermaid_er_reference_svg() {
        // Read the actual mermaid reference SVG file if it exists
        let path = "docs/images/er.svg";
        if !std::path::Path::new(path).exists() {
            eprintln!("Skipping test: {} not found", path);
            return;
        }

        let svg = std::fs::read_to_string(path).unwrap();

        // First check how many data-edge attributes we can find in the raw string
        let data_edge_count = svg.matches("data-edge").count();
        eprintln!("Raw data-edge count in file: {}", data_edge_count);

        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("edge_count: {}", structure.edge_count);
        eprintln!(
            "edge_endpoints: {:?}",
            structure.edge_geometry.edge_endpoints
        );
        eprintln!(
            "edge_details: {:?}",
            structure.edge_geometry.edge_details.len()
        );

        // Should detect edges if data-edge is present
        if data_edge_count > 0 {
            assert!(
                structure.edge_count > 0,
                "Expected edges to be detected, got edge_count={}",
                structure.edge_count
            );
        }
    }

    #[test]
    fn test_mermaid_reference_from_eval_report() {
        // Try to find and read a mermaid reference SVG from the eval-report directory
        // This tests the actual mermaid-rendered SVG, not the selkie output
        let pattern = "eval-report/selkie-eval-*/er/er_reference.svg";
        let paths: Vec<_> = glob::glob(pattern)
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .collect();

        if paths.is_empty() {
            eprintln!("Skipping test: no eval-report reference SVG found");
            return;
        }

        let path = &paths[0];
        eprintln!("Testing mermaid reference: {}", path.display());

        let svg = std::fs::read_to_string(path).unwrap();

        // Count raw data-edge occurrences in the file
        let data_edge_count = svg.matches("data-edge=\"true\"").count();
        eprintln!("Raw data-edge=\"true\" count: {}", data_edge_count);

        // Parse the structure
        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("Parsed edge_count: {}", structure.edge_count);
        eprintln!(
            "Edge endpoints: {}",
            structure.edge_geometry.edge_endpoints.len()
        );

        // Mermaid ER diagrams should have edges detected via data-edge attribute
        assert_eq!(
            structure.edge_count, data_edge_count,
            "Edge count ({}) should match data-edge count ({})",
            structure.edge_count, data_edge_count
        );
    }

    #[test]
    fn test_selkie_er_svg_edge_attachment_detection() {
        // Test the actual selkie-generated ER SVG to trace edge attachment detection
        let pattern = "eval-report/selkie-eval-*/er/er_selkie.svg";
        let mut paths: Vec<_> = glob::glob(pattern)
            .expect("Failed to read glob pattern")
            .filter_map(|r| r.ok())
            .collect();

        if paths.is_empty() {
            eprintln!("Skipping test: no selkie SVG found");
            return;
        }

        // Sort by modification time to get the most recent
        paths.sort_by(|a, b| {
            let a_time = std::fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = std::fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time) // Reverse order: most recent first
        });
        let path = &paths[0]; // Use the most recent
        eprintln!("Testing selkie SVG: {}", path.display());

        let svg = std::fs::read_to_string(path).unwrap();
        let structure = SvgStructure::from_svg(&svg).unwrap();

        eprintln!("node_count: {}", structure.node_count);
        eprintln!("edge_count: {}", structure.edge_count);
        eprintln!(
            "node_bounds count: {}",
            structure.edge_geometry.node_bounds.len()
        );

        // Print all node bounds
        for (i, bounds) in structure.edge_geometry.node_bounds.iter().enumerate() {
            eprintln!(
                "Node bounds {}: id={} x={:.1} y={:.1} w={:.1} h={:.1}",
                i, bounds.id, bounds.x, bounds.y, bounds.width, bounds.height
            );
        }

        // Print edge details
        eprintln!(
            "edge_endpoints count: {}",
            structure.edge_geometry.edge_endpoints.len()
        );
        for (i, (sx, sy, ex, ey)) in structure.edge_geometry.edge_endpoints.iter().enumerate() {
            eprintln!(
                "Edge endpoint {}: ({:.1}, {:.1}) → ({:.1}, {:.1})",
                i, sx, sy, ex, ey
            );
        }

        for (i, detail) in structure.edge_geometry.edge_details.iter().enumerate() {
            eprintln!(
                "Edge detail {}: start_edge={} end_edge={} start_offset={:.1} end_offset={:.1}",
                i,
                detail.start_edge,
                detail.end_edge,
                detail.start_center_offset,
                detail.end_center_offset
            );
        }

        // The rendering is correct - let's verify the coordinates
        // Edge 2 should end at LINE-ITEM's LEFT side (x=175.05)
        // Edge 3 should end at LINE-ITEM's RIGHT side (x=304.95)
        if structure.edge_geometry.edge_endpoints.len() >= 3 {
            let edge2 = &structure.edge_geometry.edge_endpoints[1];
            let edge3 = &structure.edge_geometry.edge_endpoints[2];

            // Edge 2 endpoint (end_x, end_y)
            let (_, _, end_x2, end_y2) = edge2;
            // Edge 3 endpoint (end_x, end_y)
            let (_, _, end_x3, end_y3) = edge3;

            eprintln!("Edge 2 end: ({:.2}, {:.2})", end_x2, end_y2);
            eprintln!("Edge 3 end: ({:.2}, {:.2})", end_x3, end_y3);

            // Find LINE-ITEM bounds
            let line_item_bounds = structure
                .edge_geometry
                .node_bounds
                .iter()
                .find(|b| b.id.contains("LINE-ITEM") || b.x > 150.0 && b.y > 500.0);

            if let Some(bounds) = line_item_bounds {
                eprintln!(
                    "LINE-ITEM bounds: x={:.1} y={:.1} w={:.1} h={:.1}",
                    bounds.x, bounds.y, bounds.width, bounds.height
                );

                // Check if edge 2 ends at left side
                let dist_left = (*end_x2 - bounds.x).abs();
                let dist_right = (*end_x2 - (bounds.x + bounds.width)).abs();
                eprintln!(
                    "Edge 2 distance from left={:.1}, right={:.1}",
                    dist_left, dist_right
                );

                // Check if edge 3 ends at right side
                let dist_left3 = (*end_x3 - bounds.x).abs();
                let dist_right3 = (*end_x3 - (bounds.x + bounds.width)).abs();
                eprintln!(
                    "Edge 3 distance from left={:.1}, right={:.1}",
                    dist_left3, dist_right3
                );
            }
        }
    }

    #[test]
    fn test_extract_labels_from_foreignobject_with_br() {
        // Mermaid.js uses foreignObject with <p> and <br> for multi-line text.
        // The label extractor must collect ALL text content from <p> elements,
        // not just the first text node before a <br/>.
        let mermaid_html_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 500 200">
            <g class="node">
                <foreignObject width="200" height="100">
                    <div xmlns="http://www.w3.org/1999/xhtml">
                        <span class="nodeLabel"><p>Inner / circle<br/>and some odd <br/>special characters</p></span>
                    </div>
                </foreignObject>
            </g>
        </svg>"#;

        let structure = SvgStructure::from_svg(mermaid_html_svg).unwrap();

        // Should extract the full multi-line text, not just "Inner / circle"
        let has_full_text = structure
            .labels
            .iter()
            .any(|l| l.contains("Inner / circle") && l.contains("special characters"));
        assert!(
            has_full_text,
            "Should extract full text from <p> with <br/> tags. Got: {:?}",
            structure.labels
        );

        // Should NOT have a partial label like just "Inner / circle"
        let has_partial = structure.labels.iter().any(|l| l == "Inner / circle");
        assert!(
            !has_partial,
            "Should not extract partial label from first text node only. Got: {:?}",
            structure.labels
        );
    }
}
