//! Position overrides parsing for mermaid diagrams
//!
//! Parses `%% selkie:positions {...} %%` comments that specify manual node positions.
//!
//! ## Supported syntax
//!
//! ```text
//! %% selkie:positions {"A": {"x": 100, "y": 200}, "B": {"x": 300, "y": 400}} %%
//! ```
//!
//! The positions comment can appear anywhere in the diagram text.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Regex to match `%% selkie:positions {...} %%` comments
static POSITIONS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%%\s*selkie:positions\s+(\{[^%]*\})\s*%%").unwrap());

/// A single node position override
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

impl NodePosition {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Position overrides for multiple nodes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionOverrides {
    /// Map of node ID to position
    #[serde(flatten)]
    pub positions: HashMap<String, NodePosition>,
}

impl PositionOverrides {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any position overrides
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Get the position override for a node
    pub fn get(&self, node_id: &str) -> Option<&NodePosition> {
        self.positions.get(node_id)
    }

    /// Set a position override for a node
    pub fn set(&mut self, node_id: impl Into<String>, x: f64, y: f64) {
        self.positions
            .insert(node_id.into(), NodePosition::new(x, y));
    }

    /// Remove a position override for a node
    pub fn remove(&mut self, node_id: &str) -> Option<NodePosition> {
        self.positions.remove(node_id)
    }
}

/// Detect and parse position overrides from diagram text
///
/// Returns position overrides if found, or None if not present.
pub fn detect_positions(text: &str) -> Option<PositionOverrides> {
    let cap = POSITIONS_REGEX.captures(text)?;
    let json_content = cap.get(1)?.as_str();

    // Parse as JSON
    let positions: HashMap<String, NodePosition> = serde_json::from_str(json_content).ok()?;

    if positions.is_empty() {
        return None;
    }

    Some(PositionOverrides { positions })
}

/// Remove position comments from diagram text
///
/// Returns the text with all `%% selkie:positions {...} %%` comments stripped out.
pub fn remove_positions(text: &str) -> String {
    POSITIONS_REGEX.replace_all(text, "").trim().to_string()
}

/// Encode position overrides as a comment string
///
/// Returns a string like `%% selkie:positions {"A": {"x": 100, "y": 200}} %%`
pub fn encode_positions(overrides: &PositionOverrides) -> String {
    if overrides.is_empty() {
        return String::new();
    }

    let json = serde_json::to_string(&overrides.positions).unwrap_or_default();
    format!("%% selkie:positions {} %%", json)
}

/// Update position overrides in diagram text
///
/// Replaces existing position comment or appends a new one.
/// If overrides are empty, removes the position comment.
pub fn update_positions(text: &str, overrides: &PositionOverrides) -> String {
    let text_without_positions = remove_positions(text);

    if overrides.is_empty() {
        return text_without_positions;
    }

    let positions_comment = encode_positions(overrides);
    format!("{}\n{}", positions_comment, text_without_positions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_positions() {
        let text = r#"%% selkie:positions {"A": {"x": 100, "y": 200}, "B": {"x": 300, "y": 400}} %%
flowchart TD
    A --> B"#;

        let overrides = detect_positions(text).expect("Should parse positions");
        assert_eq!(overrides.positions.len(), 2);

        let a_pos = overrides.get("A").expect("Should have A");
        assert_eq!(a_pos.x, 100.0);
        assert_eq!(a_pos.y, 200.0);

        let b_pos = overrides.get("B").expect("Should have B");
        assert_eq!(b_pos.x, 300.0);
        assert_eq!(b_pos.y, 400.0);
    }

    #[test]
    fn test_detect_positions_no_positions() {
        let text = r#"flowchart TD
    A --> B"#;

        assert!(detect_positions(text).is_none());
    }

    #[test]
    fn test_remove_positions() {
        let text = r#"%% selkie:positions {"A": {"x": 100, "y": 200}} %%
flowchart TD
    A --> B"#;

        let cleaned = remove_positions(text);
        assert!(!cleaned.contains("selkie:positions"));
        assert!(cleaned.contains("flowchart TD"));
    }

    #[test]
    fn test_encode_positions() {
        let mut overrides = PositionOverrides::new();
        overrides.set("A", 100.0, 200.0);

        let encoded = encode_positions(&overrides);
        assert!(encoded.contains("selkie:positions"));
        assert!(encoded.contains("\"A\""));
        assert!(encoded.contains("100"));
        assert!(encoded.contains("200"));
    }

    #[test]
    fn test_encode_empty_positions() {
        let overrides = PositionOverrides::new();
        let encoded = encode_positions(&overrides);
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_update_positions_add() {
        let text = r#"flowchart TD
    A --> B"#;

        let mut overrides = PositionOverrides::new();
        overrides.set("A", 100.0, 200.0);

        let updated = update_positions(text, &overrides);
        assert!(updated.contains("selkie:positions"));
        assert!(updated.contains("flowchart TD"));
    }

    #[test]
    fn test_update_positions_replace() {
        let text = r#"%% selkie:positions {"A": {"x": 50, "y": 50}} %%
flowchart TD
    A --> B"#;

        let mut overrides = PositionOverrides::new();
        overrides.set("A", 100.0, 200.0);

        let updated = update_positions(text, &overrides);

        // Should have exactly one positions comment
        assert_eq!(
            updated.matches("selkie:positions").count(),
            1,
            "Should have exactly one positions comment"
        );

        // Should contain new coordinates
        assert!(updated.contains("100"));
        assert!(updated.contains("200"));
    }

    #[test]
    fn test_update_positions_remove() {
        let text = r#"%% selkie:positions {"A": {"x": 100, "y": 200}} %%
flowchart TD
    A --> B"#;

        let overrides = PositionOverrides::new();
        let updated = update_positions(text, &overrides);

        assert!(!updated.contains("selkie:positions"));
        assert!(updated.contains("flowchart TD"));
    }

    #[test]
    fn test_roundtrip() {
        let mut overrides = PositionOverrides::new();
        overrides.set("node1", 123.5, 456.7);
        overrides.set("node2", -10.0, 20.0);

        let encoded = encode_positions(&overrides);
        let parsed = detect_positions(&encoded).expect("Should parse encoded positions");

        assert_eq!(parsed.positions.len(), 2);
        assert_eq!(parsed.get("node1").unwrap().x, 123.5);
        assert_eq!(parsed.get("node1").unwrap().y, 456.7);
        assert_eq!(parsed.get("node2").unwrap().x, -10.0);
        assert_eq!(parsed.get("node2").unwrap().y, 20.0);
    }
}
