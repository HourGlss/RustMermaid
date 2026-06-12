//! Mindmap parser

use regex::Regex;
use std::sync::LazyLock;

use super::types::{MindmapDb, MindmapNode, NodeType};
use crate::error::{MermaidError, Result};

// Regex patterns for mindmap parsing
static MINDMAP_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*mindmap\s*$").unwrap());

static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%%.*$").unwrap());

// Node patterns with different shapes
static NODE_RECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^(\w+)?\[(?:"([^"]+)"|([^\]]+))\]$"#).unwrap());

static NODE_ROUNDED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)?\(([^)]+)\)$").unwrap());

static NODE_CIRCLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)?\(\(([^)]+)\)\)$").unwrap());

static NODE_CLOUD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)?\)([^(]+)\($").unwrap());

static NODE_BANG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)?\)\)([^(]+)\(\($").unwrap());

static NODE_HEXAGON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)?\{\{([^}]+)\}\}$").unwrap());

// Icon classes can contain multiple space-separated words with hyphens (e.g., "fa fa-book")
static ICON_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^::icon\(([^)]+)\)$").unwrap());

static CLASS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:::(.+)$").unwrap());

/// Parse a mindmap diagram
pub fn parse(input: &str) -> Result<MindmapDb> {
    let mut db = MindmapDb::new();
    parse_into(input, &mut db)?;
    Ok(db)
}

/// Parse into an existing database
pub fn parse_into(input: &str, db: &mut MindmapDb) -> Result<()> {
    db.clear();

    let lines: Vec<&str> = input.lines().collect();
    let mut i = find_mindmap_body_start(&lines);

    // Parse nodes with indentation-based hierarchy
    let mut stack: Vec<(usize, MindmapNode)> = Vec::new(); // (indent, node)
    let mut pending_decorations: Option<usize> = None; // index in stack

    while i < lines.len() {
        let line = lines[i];

        // Remove inline comments
        let line = COMMENT_RE.replace(line, "").to_string();

        // Skip empty/whitespace-only lines
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Calculate indentation (number of leading spaces)
        let indent = line.len() - line.trim_start().len();
        let content = line.trim();

        if apply_decoration(content, &mut stack, pending_decorations) {
            i += 1;
            continue;
        }

        // Parse node
        let node = parse_node(content)?;

        // Find parent based on indentation
        attach_popped_nodes(indent, &mut stack, db)?;

        // Check for multiple roots
        if stack.is_empty() && db.get_mindmap().is_some() {
            return Err(MermaidError::ParseError(format!(
                "There can be only one root. No parent could be found for (\"{}\")",
                node.descr
            )));
        }

        pending_decorations = Some(stack.len());
        stack.push((indent, node));
        i += 1;
    }

    collapse_stack(&mut stack, db);

    Ok(())
}

fn find_mindmap_body_start(lines: &[&str]) -> usize {
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() || COMMENT_RE.is_match(line) {
            i += 1;
            continue;
        }
        if MINDMAP_HEADER_RE.is_match(line) {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn apply_decoration(
    content: &str,
    stack: &mut [(usize, MindmapNode)],
    pending_decorations: Option<usize>,
) -> bool {
    if let Some(caps) = ICON_RE.captures(content) {
        apply_to_decoration_target(stack, pending_decorations, |node| {
            node.set_icon(caps.get(1).unwrap().as_str());
        });
        return true;
    }

    if let Some(caps) = CLASS_RE.captures(content) {
        apply_to_decoration_target(stack, pending_decorations, |node| {
            node.set_class(caps.get(1).unwrap().as_str());
        });
        return true;
    }

    false
}

fn apply_to_decoration_target<F>(
    stack: &mut [(usize, MindmapNode)],
    pending_decorations: Option<usize>,
    mut apply: F,
) where
    F: FnMut(&mut MindmapNode),
{
    let target_idx = pending_decorations.or_else(|| stack.last().map(|_| stack.len() - 1));
    if let Some(target_idx) = target_idx {
        if let Some((_, node)) = stack.get_mut(target_idx) {
            apply(node);
        }
    }
}

fn attach_popped_nodes(
    indent: usize,
    stack: &mut Vec<(usize, MindmapNode)>,
    db: &mut MindmapDb,
) -> Result<()> {
    while let Some((parent_indent, _)) = stack.last() {
        if *parent_indent < indent {
            break;
        }
        attach_stack_child(stack, db)?;
    }
    Ok(())
}

fn attach_stack_child(stack: &mut Vec<(usize, MindmapNode)>, db: &mut MindmapDb) -> Result<()> {
    let (_, child) = stack.pop().unwrap();
    if let Some((_, parent)) = stack.last_mut() {
        parent.add_child(child);
    } else if db.get_mindmap().is_some() {
        return Err(MermaidError::ParseError(format!(
            "There can be only one root. No parent could be found for (\"{}\")",
            child.descr
        )));
    } else {
        db.set_root(child);
    }
    Ok(())
}

fn collapse_stack(stack: &mut Vec<(usize, MindmapNode)>, db: &mut MindmapDb) {
    while let Some((_, child)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.add_child(child);
        } else {
            db.set_root(child);
        }
    }
}

/// Parse a single node from content
fn parse_node(content: &str) -> Result<MindmapNode> {
    // Try different node patterns in order of specificity

    // Circle: ((text))
    if let Some(caps) = NODE_CIRCLE_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps.get(2).unwrap().as_str().to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::Circle,
            ..Default::default()
        });
    }

    // Bang: ))text((
    if let Some(caps) = NODE_BANG_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps.get(2).unwrap().as_str().to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::Bang,
            ..Default::default()
        });
    }

    // Hexagon: {{text}}
    if let Some(caps) = NODE_HEXAGON_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps.get(2).unwrap().as_str().to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::Hexagon,
            ..Default::default()
        });
    }

    // Cloud: )text(
    if let Some(caps) = NODE_CLOUD_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps.get(2).unwrap().as_str().to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::Cloud,
            ..Default::default()
        });
    }

    // Rect: [text] or ["text"]
    if let Some(caps) = NODE_RECT_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps
            .get(2)
            .or_else(|| caps.get(3))
            .unwrap()
            .as_str()
            .to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::Rect,
            ..Default::default()
        });
    }

    // Rounded: (text)
    if let Some(caps) = NODE_ROUNDED_RE.captures(content) {
        let id = caps.get(1).map(|m| m.as_str().to_string());
        let descr = caps.get(2).unwrap().as_str().to_string();
        return Ok(MindmapNode {
            node_id: id,
            descr,
            node_type: NodeType::RoundedRect,
            ..Default::default()
        });
    }

    // Plain node (just text, optionally an ID)
    let node_id = if content.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Some(content.to_string())
    } else {
        None
    };

    Ok(MindmapNode {
        node_id,
        descr: content.to_string(),
        node_type: NodeType::Default,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests ported from mermaid Cypress tests (mindmap-tidy-tree.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_simple_mindmap_without_children() {
            // From Cypress 1-tidy-tree: should render a simple mindmap without children
            let input = r#"mindmap
      root((mindmap))
        A
        B"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_simple_mindmap() {
            // From Cypress 2-tidy-tree: should render a simple mindmap
            let input = r#"mindmap
      root((mindmap is a long thing))
        A
        B
        C
        D"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_mindmap_different_shapes() {
            // From Cypress 3-tidy-tree: should render a mindmap with different shapes
            let input = r#"mindmap
      root((mindmap))
        Origins
          Long history
          Popularisation
            British popular psychology author Tony Buzan
        Research
          On effectiveness and features
          On Automatic creation
            Uses
                Creative techniques
                Strategic planning
                Argument mapping
        Tools"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_mindmap_with_children() {
            // From Cypress 4-tidy-tree: should render a mindmap with children
            let input = r#"mindmap
      ((This is a mindmap))
        child1
         grandchild 1
         grandchild 2
        child2
         grandchild 3
         grandchild 4
        child3
         grandchild 5
         grandchild 6"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }
    }
}
