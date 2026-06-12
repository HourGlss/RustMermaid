//! ASCII renderer for Sankey flow diagrams.
//!
//! Renders Sankey diagrams as column-based flow layouts showing source nodes
//! on the left, target nodes on the right, with proportional flow bands
//! connecting them. Each flow band occupies a number of rows proportional
//! to the link value, using block characters for visual density.

use std::collections::{HashMap, VecDeque};

use crate::diagrams::sankey::SankeyDb;
use crate::error::Result;

/// Total number of content rows for the flow area.
const TOTAL_ROWS: usize = 20;
/// Width of the flow area in characters.
const FLOW_WIDTH: usize = 30;
/// Block character for drawing flow bands.
const FULL_BLOCK: char = '█';

/// A flow band with its row allocation and display width.
#[derive(Debug, Clone)]
struct FlowBand {
    target_id: String,
    value: f64,
    /// Start row (inclusive).
    row_start: usize,
    /// End row (exclusive).
    row_end: usize,
    /// Width in block characters (proportional to value).
    bar_width: usize,
}

/// Render a Sankey diagram as character art.
pub fn render_sankey_ascii(db: &SankeyDb) -> Result<String> {
    let links = db.get_links();
    if links.is_empty() {
        return Ok("(empty sankey diagram)\n".to_string());
    }

    let graph = db.get_graph();
    if graph.nodes.is_empty() {
        return Ok("(empty sankey diagram)\n".to_string());
    }

    // Build adjacency info
    let mut outgoing: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<(String, f64)>> = HashMap::new();

    for link in &graph.links {
        outgoing
            .entry(link.source.clone())
            .or_default()
            .push((link.target.clone(), link.value));
        incoming
            .entry(link.target.clone())
            .or_default()
            .push((link.source.clone(), link.value));
    }

    // Compute node columns
    let node_columns = compute_columns(&graph.nodes, &outgoing, &incoming);
    let max_column = node_columns.values().copied().max().unwrap_or(0);

    // Group nodes by column
    let mut nodes_by_column: Vec<Vec<String>> = vec![Vec::new(); max_column + 1];
    for node in &graph.nodes {
        let col = node_columns.get(&node.id).copied().unwrap_or(0);
        nodes_by_column[col].push(node.id.clone());
    }

    // Compute node values
    let node_values = compute_node_values(&graph.nodes, &graph.links);

    // Single column: just list nodes
    if max_column == 0 {
        let mut lines = Vec::new();
        for node_id in &nodes_by_column[0] {
            let value = node_values.get(node_id).copied().unwrap_or(0.0);
            lines.push(format!("  {} [{}]", node_id, format_value(value)));
        }
        lines.push(String::new());
        return Ok(lines.join("\n"));
    }

    // Find all unique column pairs (source_col, target_col) from links
    let mut column_pairs: Vec<(usize, usize)> = Vec::new();
    for link in &graph.links {
        let src_col = node_columns.get(&link.source).copied().unwrap_or(0);
        let tgt_col = node_columns.get(&link.target).copied().unwrap_or(0);
        let pair = (src_col, tgt_col);
        if !column_pairs.contains(&pair) {
            column_pairs.push(pair);
        }
    }
    column_pairs.sort();

    let mut all_lines = Vec::new();
    let mut first_pair = true;

    for (left_col, right_col) in &column_pairs {
        let left_nodes = &nodes_by_column[*left_col];
        let right_nodes = &nodes_by_column[*right_col];

        // Collect links between this column pair
        let pair_links: Vec<_> = graph
            .links
            .iter()
            .filter(|l| {
                node_columns.get(&l.source).copied() == Some(*left_col)
                    && node_columns.get(&l.target).copied() == Some(*right_col)
            })
            .collect();

        if pair_links.is_empty() {
            continue;
        }

        if !first_pair {
            all_lines.push(String::new());
        }
        first_pair = false;

        let lines = render_column_pair(
            left_nodes,
            right_nodes,
            &pair_links
                .iter()
                .map(|l| (l.source.as_str(), l.target.as_str(), l.value))
                .collect::<Vec<_>>(),
            &node_values,
        );
        all_lines.extend(lines);
    }

    all_lines.push(String::new());
    Ok(all_lines.join("\n"))
}

/// Render a pair of adjacent columns with flow bands between them.
fn render_column_pair(
    left_nodes: &[String],
    _right_nodes: &[String],
    links: &[(&str, &str, f64)],
    node_values: &HashMap<String, f64>,
) -> Vec<String> {
    let layout = ColumnPairLayout::new(left_nodes, links, node_values);
    let mut lines = Vec::new();
    for row in 0..layout.total_rows {
        lines.push(render_column_pair_row(row, &layout));
    }
    lines
}

struct ColumnPairLayout {
    bands: Vec<FlowBand>,
    left_node_ranges: HashMap<String, (usize, usize)>,
    left_labels: HashMap<usize, String>,
    right_labels: HashMap<usize, (String, String)>,
    left_label_width: usize,
    total_rows: usize,
}

impl ColumnPairLayout {
    fn new(
        left_nodes: &[String],
        links: &[(&str, &str, f64)],
        node_values: &HashMap<String, f64>,
    ) -> Self {
        let flows_by_source = group_flows_by_source(links);
        let bands = compute_flow_bands(links);
        let total_rows = bands.last().map(|band| band.row_end).unwrap_or(0);
        let left_node_ranges = compute_left_node_ranges(&flows_by_source, &bands, links);
        let left_label_width = left_label_width(left_nodes);
        let left_labels =
            compute_left_labels(left_nodes, &left_node_ranges, node_values, total_rows);
        let right_labels = compute_right_labels(&bands, node_values);

        Self {
            bands,
            left_node_ranges,
            left_labels,
            right_labels,
            left_label_width,
            total_rows,
        }
    }
}

fn group_flows_by_source<'a>(
    links: &[(&'a str, &'a str, f64)],
) -> Vec<(String, Vec<(&'a str, f64)>)> {
    let mut flows_by_source: Vec<(String, Vec<(&'a str, f64)>)> = Vec::new();
    let mut source_order: Vec<String> = Vec::new();
    for (src, _, _) in links {
        if !source_order.contains(&src.to_string()) {
            source_order.push(src.to_string());
        }
    }
    for src in &source_order {
        let src_links = links
            .iter()
            .filter(|(s, _, _)| *s == src.as_str())
            .map(|(_, t, v)| (*t, *v))
            .collect();
        flows_by_source.push((src.clone(), src_links));
    }
    flows_by_source
}

fn compute_flow_bands(links: &[(&str, &str, f64)]) -> Vec<FlowBand> {
    let max_flow_value = links.iter().map(|(_, _, v)| *v).fold(0.0f64, f64::max);
    let total_flow: f64 = links.iter().map(|(_, _, v)| *v).sum();
    let num_flows = links.len();
    let scaled_total = scaled_total_rows(num_flows);
    let gap_rows = num_flows.saturating_sub(1);
    let available_rows = scaled_total.saturating_sub(gap_rows);

    let mut bands: Vec<FlowBand> = Vec::new();
    let mut current_row = 0;

    for (i, (_src, tgt, val)) in links.iter().enumerate() {
        bands.push(FlowBand {
            target_id: tgt.to_string(),
            value: *val,
            row_start: current_row,
            row_end: current_row + flow_rows(*val, total_flow, available_rows),
            bar_width: flow_bar_width(*val, max_flow_value),
        });

        current_row = bands.last().map(|band| band.row_end).unwrap_or(current_row);
        if i < num_flows - 1 {
            current_row += 1;
        }
    }

    bands
}

fn scaled_total_rows(num_flows: usize) -> usize {
    if num_flows <= 1 {
        6
    } else if num_flows <= 2 {
        12
    } else {
        TOTAL_ROWS
    }
}

fn flow_rows(value: f64, total_flow: f64, available_rows: usize) -> usize {
    if total_flow > 0.0 {
        ((value / total_flow) * available_rows as f64).round() as usize
    } else {
        1
    }
    .max(1)
}

fn flow_bar_width(value: f64, max_flow_value: f64) -> usize {
    if max_flow_value > 0.0 {
        ((value / max_flow_value) * FLOW_WIDTH as f64).round() as usize
    } else {
        1
    }
    .clamp(1, FLOW_WIDTH)
}

fn compute_left_node_ranges(
    flows_by_source: &[(String, Vec<(&str, f64)>)],
    bands: &[FlowBand],
    links: &[(&str, &str, f64)],
) -> HashMap<String, (usize, usize)> {
    let mut left_node_ranges: HashMap<String, (usize, usize)> = HashMap::new();
    for (src, _src_links) in flows_by_source {
        let src_bands: Vec<_> = bands
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                links
                    .iter()
                    .any(|(s, t, _)| *s == src.as_str() && *t == b.target_id.as_str())
            })
            .collect();

        if let (Some(first), Some(last)) = (src_bands.first(), src_bands.last()) {
            left_node_ranges.insert(src.clone(), (first.1.row_start, last.1.row_end));
        }
    }
    left_node_ranges
}

fn left_label_width(left_nodes: &[String]) -> usize {
    left_nodes
        .iter()
        .map(|id| id.chars().count())
        .max()
        .unwrap_or(3)
        .max(3)
}

fn render_column_pair_row(row: usize, layout: &ColumnPairLayout) -> String {
    let mut line = String::new();
    push_left_label(&mut line, row, layout);
    push_left_border(&mut line, row, layout);
    let active_band = layout
        .bands
        .iter()
        .find(|b| row >= b.row_start && row < b.row_end);
    let is_gap_row = active_band.is_none() && row > 0 && row < layout.total_rows;
    push_flow_area(&mut line, active_band, is_gap_row);
    line.push(if active_band.is_some() || is_gap_row {
        '│'
    } else {
        ' '
    });
    if let Some((name, val_str)) = layout.right_labels.get(&row) {
        line.push_str(&format!(" {} {}", name, val_str));
    }
    line
}

fn push_left_label(line: &mut String, row: usize, layout: &ColumnPairLayout) {
    if let Some(label_text) = layout.left_labels.get(&row) {
        line.push_str(&format!(
            "{:>width$} ",
            label_text,
            width = layout.left_label_width
        ));
    } else {
        line.push_str(&format!("{:>width$} ", "", width = layout.left_label_width));
    }
}

fn push_left_border(line: &mut String, row: usize, layout: &ColumnPairLayout) {
    let in_left_node = layout
        .left_node_ranges
        .values()
        .any(|(start, end)| row >= *start && row < *end);
    line.push(if in_left_node { '│' } else { ' ' });
}

fn push_flow_area(line: &mut String, active_band: Option<&FlowBand>, is_gap_row: bool) {
    if let Some(band) = active_band {
        let mut flow_chars: Vec<char> = vec![' '; FLOW_WIDTH];
        for ch in flow_chars.iter_mut().take(band.bar_width) {
            *ch = FULL_BLOCK;
        }
        line.push_str(&flow_chars.into_iter().collect::<String>());
    } else if is_gap_row {
        line.push_str(&"─".repeat(FLOW_WIDTH));
    } else {
        line.push_str(&" ".repeat(FLOW_WIDTH));
    }
}

/// Compute left-side labels: node name on middle row, value on next row.
fn compute_left_labels(
    left_nodes: &[String],
    left_node_ranges: &HashMap<String, (usize, usize)>,
    node_values: &HashMap<String, f64>,
    _total_rows: usize,
) -> HashMap<usize, String> {
    let mut labels = HashMap::new();

    for node_id in left_nodes {
        if let Some((start, end)) = left_node_ranges.get(node_id) {
            let height = end - start;
            let mid_row = start + height / 2;
            labels.insert(mid_row, node_id.clone());

            // Show value on the row below if there's space
            let value = node_values.get(node_id).copied().unwrap_or(0.0);
            let val_row = if height >= 3 { mid_row + 1 } else { mid_row };
            if val_row != mid_row {
                labels.insert(val_row, format_value(value));
            }
        }
    }

    labels
}

/// Compute right-side labels: target name and value at the middle of each band.
fn compute_right_labels(
    bands: &[FlowBand],
    _node_values: &HashMap<String, f64>,
) -> HashMap<usize, (String, String)> {
    let mut labels = HashMap::new();

    for band in bands {
        let mid_row = band.row_start + (band.row_end - band.row_start) / 2;
        labels.insert(mid_row, (band.target_id.clone(), format_value(band.value)));
    }

    labels
}

/// Compute node columns using topological depth from sources.
fn compute_columns(
    nodes: &[crate::diagrams::sankey::GraphNode],
    outgoing: &HashMap<String, Vec<(String, f64)>>,
    incoming: &HashMap<String, Vec<(String, f64)>>,
) -> HashMap<String, usize> {
    let mut columns: HashMap<String, usize> = HashMap::new();

    let source_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| !incoming.contains_key(&n.id) || incoming.get(&n.id).unwrap().is_empty())
        .map(|n| n.id.clone())
        .collect();

    let mut queue: VecDeque<(String, usize)> =
        source_nodes.iter().map(|id| (id.clone(), 0)).collect();

    while let Some((node_id, col)) = queue.pop_front() {
        let current_col = columns.entry(node_id.clone()).or_insert(0);
        if col > *current_col {
            *current_col = col;
        }

        if let Some(edges) = outgoing.get(&node_id) {
            for (target, _) in edges {
                queue.push_back((target.clone(), col + 1));
            }
        }
    }

    for node in nodes {
        columns.entry(node.id.clone()).or_insert(0);
    }

    // Justify: push sink nodes to rightmost column
    let max_column = columns.values().copied().max().unwrap_or(0);
    for node in nodes {
        let has_outgoing = outgoing
            .get(&node.id)
            .map(|edges| !edges.is_empty())
            .unwrap_or(false);
        if !has_outgoing {
            columns.insert(node.id.clone(), max_column);
        }
    }

    columns
}

/// Compute total flow through each node (max of incoming/outgoing).
fn compute_node_values(
    nodes: &[crate::diagrams::sankey::GraphNode],
    links: &[crate::diagrams::sankey::GraphLink],
) -> HashMap<String, f64> {
    let mut incoming_values: HashMap<String, f64> = HashMap::new();
    let mut outgoing_values: HashMap<String, f64> = HashMap::new();

    for link in links {
        *incoming_values.entry(link.target.clone()).or_insert(0.0) += link.value;
        *outgoing_values.entry(link.source.clone()).or_insert(0.0) += link.value;
    }

    let mut values: HashMap<String, f64> = HashMap::new();
    for node in nodes {
        let inc = incoming_values.get(&node.id).copied().unwrap_or(0.0);
        let out = outgoing_values.get(&node.id).copied().unwrap_or(0.0);
        values.insert(node.id.clone(), inc.max(out));
    }

    values
}

/// Format a value for display.
fn format_value(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        let s = format!("{:.2}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sankey() {
        let db = SankeyDb::new();
        let output = render_sankey_ascii(&db).unwrap();
        assert!(output.contains("empty sankey"));
    }

    #[test]
    fn gallery_sankey_renders() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_ascii(&db).unwrap();
        assert!(output.contains("Revenue"), "Output:\n{}", output);
        assert!(output.contains("Salaries"), "Output:\n{}", output);
        assert!(output.contains("Operations"), "Output:\n{}", output);
    }

    #[test]
    fn has_flow_structure() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_ascii(&db).unwrap();
        let block_count = output.chars().filter(|&c| c == FULL_BLOCK).count();
        assert!(
            block_count > 20,
            "Should have many block characters for flow bands, got {}\nOutput:\n{}",
            block_count,
            output
        );
    }

    #[test]
    fn node_separator_bars_present() {
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_ascii(&db).unwrap();
        let bar_count = output.chars().filter(|&c| c == '│').count();
        assert!(
            bar_count >= 4,
            "Should have vertical bars for node columns, got {}\nOutput:\n{}",
            bar_count,
            output
        );
    }

    #[test]
    fn flow_bands_proportional_height() {
        // Larger flows should occupy more rows than smaller flows
        let mut db = SankeyDb::new();
        db.add_link("A", "Big", 80.0);
        db.add_link("A", "Small", 20.0);

        let output = render_sankey_ascii(&db).unwrap();

        // Both targets should appear
        assert!(output.contains("Big"), "Output:\n{}", output);
        assert!(output.contains("Small"), "Output:\n{}", output);

        // Should have substantial flow rendering
        let total_blocks: usize = output
            .lines()
            .map(|l| l.chars().filter(|&c| c == FULL_BLOCK).count())
            .sum();
        assert!(
            total_blocks > 10,
            "Should have substantial flow rendering\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn values_displayed() {
        let mut db = SankeyDb::new();
        db.add_link("Source", "Target", 42.0);

        let output = render_sankey_ascii(&db).unwrap();
        assert!(
            output.contains("42"),
            "Should show flow value\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn chain_layout() {
        // Chain: a → b → c should show all nodes
        let mut db = SankeyDb::new();
        db.add_link("a", "b", 10.0);
        db.add_link("b", "c", 10.0);

        let output = render_sankey_ascii(&db).unwrap();
        assert!(output.contains("a"), "Output:\n{}", output);
        assert!(output.contains("b"), "Output:\n{}", output);
        assert!(output.contains("c"), "Output:\n{}", output);
    }

    #[test]
    fn multiple_rows_per_flow() {
        // A single large flow should occupy multiple rows
        let mut db = SankeyDb::new();
        db.add_link("Source", "Target", 100.0);

        let output = render_sankey_ascii(&db).unwrap();
        let block_lines = output.lines().filter(|l| l.contains(FULL_BLOCK)).count();
        assert!(
            block_lines >= 5,
            "A single flow should occupy multiple rows, got {}\nOutput:\n{}",
            block_lines,
            output
        );
    }

    #[test]
    fn no_bar_chart_format() {
        // Should NOT use the old bar chart format
        let input = std::fs::read_to_string("docs/sources/sankey.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Sankey(db) => db,
            _ => panic!("Expected sankey"),
        };
        let output = render_sankey_ascii(&db).unwrap();
        assert!(
            !output.contains("Flow Diagram"),
            "Should not use old bar chart header\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gap_rows_between_flows() {
        // Multiple flows should have gap rows separating them
        let mut db = SankeyDb::new();
        db.add_link("A", "X", 50.0);
        db.add_link("A", "Y", 50.0);

        let output = render_sankey_ascii(&db).unwrap();

        // There should be at least one row with no block characters between
        // the two flow bands (gap row)
        let block_line_indices: Vec<usize> = output
            .lines()
            .enumerate()
            .filter(|(_, l)| l.contains(FULL_BLOCK))
            .map(|(i, _)| i)
            .collect();

        // Check there's a gap (non-consecutive indices)
        let has_gap = block_line_indices.windows(2).any(|w| w[1] - w[0] > 1);
        assert!(
            has_gap,
            "Should have gap rows between flow bands\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn consistent_right_border() {
        // All flow rows should have consistent right border
        let mut db = SankeyDb::new();
        db.add_link("A", "X", 50.0);
        db.add_link("A", "Y", 50.0);

        let output = render_sankey_ascii(&db).unwrap();

        // Flow rows (with blocks) should have │ on the right
        for line in output.lines() {
            if line.contains(FULL_BLOCK) {
                assert!(
                    line.contains('│'),
                    "Flow rows should have border: {:?}",
                    line
                );
            }
        }
    }
}
