use crate::eval::DiagramResult;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureFamilySummary {
    pub family: String,
    pub spec_id: String,
    pub title: String,
    pub description: String,
    pub diagram_count: usize,
    pub issue_count: usize,
    pub diagrams: Vec<String>,
}

struct FamilyDefinition {
    family: &'static str,
    spec_id: &'static str,
    title: &'static str,
    description: &'static str,
}

struct FamilyAccumulator {
    definition: &'static FamilyDefinition,
    issue_count: usize,
    diagrams: BTreeSet<String>,
}

const SUBGRAPH_TITLES: FamilyDefinition = FamilyDefinition {
    family: "flow_subgraph_titles",
    spec_id: "FLOW-1.4",
    title: "Visible subgraph titles",
    description: "Reference flowcharts contain subgraph titles that Selkie omitted.",
};

const EDGE_LABELS: FamilyDefinition = FamilyDefinition {
    family: "flow_edge_labels",
    spec_id: "FLOW-2.3",
    title: "Visible edge labels",
    description: "Reference flowcharts contain edge labels that Selkie omitted.",
};

const ORIENTATION: FamilyDefinition = FamilyDefinition {
    family: "flow_orientation",
    spec_id: "FLOW-3.2",
    title: "Orientation preservation",
    description: "Rendered flowchart orientation differs materially from Mermaid.",
};

const EDGE_ROUTING: FamilyDefinition = FamilyDefinition {
    family: "flow_edge_routing",
    spec_id: "FLOW-3.3",
    title: "Edge routing preservation",
    description: "Rendered flowchart edge routes or attachment sides differ from Mermaid.",
};

const LAYOUT_SIZING: FamilyDefinition = FamilyDefinition {
    family: "flow_layout_sizing",
    spec_id: "FLOW-3.4",
    title: "Layout sizing parity",
    description: "Rendered flowchart dimensions drift from Mermaid layout sizing behavior.",
};

const VISUAL_STYLING: FamilyDefinition = FamilyDefinition {
    family: "flow_visual_styling",
    spec_id: "FLOW-4.2",
    title: "Visual styling parity",
    description: "Flowchart stroke, color, or text styling differs from Mermaid.",
};

pub fn classify_failure_families(diagrams: &[DiagramResult]) -> Vec<FailureFamilySummary> {
    let mut families: BTreeMap<&'static str, FamilyAccumulator> = BTreeMap::new();

    for diagram in diagrams {
        for issue in &diagram.issues {
            let definition = match issue.check.as_str() {
                "labels_missing" if missing_subgraph_title(diagram, &issue.message) => {
                    Some(&SUBGRAPH_TITLES)
                }
                "labels_missing" if missing_edge_label(diagram, &issue.message) => {
                    Some(&EDGE_LABELS)
                }
                "aspect_ratio" => Some(&ORIENTATION),
                "dimensions" => Some(&LAYOUT_SIZING),
                "edge_positions" | "edge_attachment_sides" => Some(&EDGE_ROUTING),
                "stroke_width" | "colors" | "text_fill_mismatch" | "text_visibility" => {
                    Some(&VISUAL_STYLING)
                }
                _ => None,
            };

            if let Some(definition) = definition {
                let entry =
                    families
                        .entry(definition.family)
                        .or_insert_with(|| FamilyAccumulator {
                            definition,
                            issue_count: 0,
                            diagrams: BTreeSet::new(),
                        });
                entry.issue_count += 1;
                entry.diagrams.insert(diagram.name.clone());
            }
        }
    }

    let mut summaries: Vec<_> = families
        .into_values()
        .map(|entry| {
            let diagrams: Vec<_> = entry.diagrams.into_iter().collect();
            FailureFamilySummary {
                family: entry.definition.family.to_string(),
                spec_id: entry.definition.spec_id.to_string(),
                title: entry.definition.title.to_string(),
                description: entry.definition.description.to_string(),
                diagram_count: diagrams.len(),
                issue_count: entry.issue_count,
                diagrams,
            }
        })
        .collect();

    summaries.sort_by(|a, b| {
        b.diagram_count
            .cmp(&a.diagram_count)
            .then_with(|| b.issue_count.cmp(&a.issue_count))
            .then_with(|| a.spec_id.cmp(&b.spec_id))
            .then_with(|| a.family.cmp(&b.family))
    });
    summaries
}

fn missing_subgraph_title(diagram: &DiagramResult, message: &str) -> bool {
    diagram
        .diagram_text
        .as_deref()
        .map(subgraph_titles)
        .unwrap_or_default()
        .iter()
        .any(|title| !title.is_empty() && message.contains(title))
}

fn missing_edge_label(diagram: &DiagramResult, message: &str) -> bool {
    diagram
        .diagram_text
        .as_deref()
        .map(edge_labels)
        .unwrap_or_default()
        .iter()
        .any(|label| !label.is_empty() && message.contains(label))
}

fn subgraph_titles(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("subgraph ")?;
            Some(extract_subgraph_title(rest.trim()))
        })
        .collect()
}

fn extract_subgraph_title(rest: &str) -> String {
    if let Some(quoted) = rest.strip_prefix('"') {
        if let Some(end) = quoted.find('"') {
            return quoted[..end].to_string();
        }
    }

    if let (Some(start), Some(end)) = (rest.find('['), rest.rfind(']')) {
        if start < end {
            return rest[start + 1..end].to_string();
        }
    }

    rest.to_string()
}

fn edge_labels(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.contains("--"))
        .flat_map(edge_labels_in_line)
        .collect()
}

fn edge_labels_in_line(line: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut remaining = line;
    while let Some(start) = remaining.find('|') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('|') else {
            break;
        };
        labels.push(after_start[..end].trim().to_string());
        remaining = &after_start[end + 1..];
    }
    labels
}

#[cfg(test)]
mod tests {
    use crate::eval::{
        failure_families::classify_failure_families, DiagramResult, Issue, Level, ParseResult,
        Status,
    };

    fn diagram(name: &str, diagram_text: &str, issues: Vec<Issue>) -> DiagramResult {
        let status = if issues.iter().any(|issue| issue.level == Level::Error) {
            Status::Error
        } else {
            Status::Warning
        };

        DiagramResult {
            name: name.to_string(),
            source: None,
            diagram_type: "flowchart".to_string(),
            diagram_text: Some(diagram_text.to_string()),
            status,
            visual_similarity: None,
            structural_similarity: None,
            structural_match: false,
            issues,
            parse_result: ParseResult {
                selkie_success: true,
                selkie_error: None,
                reference_success: true,
                reference_error: None,
            },
            render_result: None,
            selkie_svg: None,
            reference_svg: None,
        }
    }

    #[test]
    /// @spec FLOW-1.4: When a rendered flowchart reference contains subgraph titles that Selkie omits, the eval report shall group the failures under the visible subgraph titles requirement.
    fn groups_missing_subgraph_titles_under_flow_requirement() {
        let diagrams = vec![diagram(
            "subgraph_titles",
            r#"flowchart TD
subgraph "Data Collection"
    A --> B
end"#,
            vec![Issue::error(
                "labels_missing",
                r#"Missing labels: ["Data Collection"]"#,
            )],
        )];

        let families = classify_failure_families(&diagrams);

        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "flow_subgraph_titles");
        assert_eq!(families[0].spec_id, "FLOW-1.4");
        assert_eq!(families[0].diagram_count, 1);
        assert_eq!(families[0].issue_count, 1);
        assert_eq!(families[0].diagrams, vec!["subgraph_titles"]);
    }

    #[test]
    /// @spec FLOW-2.3: When Mermaid renders a flowchart edge label as layout text, the eval report shall group missing edge-label text under the edge label visibility requirement.
    fn groups_missing_edge_labels_under_flow_requirement() {
        let diagrams = vec![diagram(
            "edge_label",
            "flowchart LR\nA -->|Confirm| B",
            vec![Issue::error(
                "labels_missing",
                r#"Missing labels: ["Confirm"]"#,
            )],
        )];

        let families = classify_failure_families(&diagrams);

        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "flow_edge_labels");
        assert_eq!(families[0].spec_id, "FLOW-2.3");
        assert_eq!(families[0].diagram_count, 1);
        assert_eq!(families[0].issue_count, 1);
        assert_eq!(families[0].diagrams, vec!["edge_label"]);
    }

    #[test]
    /// @spec FLOW-3.2: When a flowchart's major rendered orientation differs from Mermaid, the eval report shall group the failure under the orientation preservation requirement.
    /// @spec FLOW-3.3: When flowchart edge routes differ from Mermaid, the eval report shall group the failure under the edge routing preservation requirement.
    fn groups_orientation_and_edge_routing_families() {
        let diagrams = vec![diagram(
            "orientation",
            "flowchart TB\nA --> B",
            vec![
                Issue::error(
                    "aspect_ratio",
                    "Diagram orientation differs: reference is square, selkie is portrait",
                ),
                Issue::warning("edge_positions", "EDGE POSITION DIFFERENCES: Edge 1"),
            ],
        )];

        let families = classify_failure_families(&diagrams);

        assert!(families
            .iter()
            .any(|family| { family.family == "flow_orientation" && family.spec_id == "FLOW-3.2" }));
        assert!(families.iter().any(|family| {
            family.family == "flow_edge_routing" && family.spec_id == "FLOW-3.3"
        }));
    }

    #[test]
    /// @spec FLOW-3.4: When Mermaid lays out a flowchart with direction and node/rank spacing, the eval report shall group dimension drift under the layout sizing requirement.
    fn groups_dimension_drift_under_layout_sizing_requirement() {
        let diagrams = vec![diagram(
            "layout_sizing",
            "flowchart TB\nA --> B",
            vec![
                Issue::warning("dimensions", "Width differs by 24%: expected 1000, got 760"),
                Issue::info("dimensions", "Height differs by 8%: expected 900, got 972"),
            ],
        )];

        let families = classify_failure_families(&diagrams);

        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "flow_layout_sizing");
        assert_eq!(families[0].spec_id, "FLOW-3.4");
        assert_eq!(families[0].diagram_count, 1);
        assert_eq!(families[0].issue_count, 2);
    }

    #[test]
    /// @spec FLOW-4.2: When flowchart styling differs from Mermaid stroke or color behavior, the eval report shall group the failure under the visual styling requirement.
    fn groups_stroke_and_color_style_mismatches() {
        let diagrams = vec![diagram(
            "style",
            "flowchart LR\nA --> B",
            vec![
                Issue::warning("stroke_width", "Edge stroke-width differs"),
                Issue::info("colors", "Color differences"),
            ],
        )];

        let families = classify_failure_families(&diagrams);

        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "flow_visual_styling");
        assert_eq!(families[0].spec_id, "FLOW-4.2");
        assert_eq!(families[0].issue_count, 2);
    }
}
