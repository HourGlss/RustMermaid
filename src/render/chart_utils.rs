//! Shared utilities for chart-type diagram renderers
//!
//! This module provides common functionality used across multiple chart types
//! (pie, radar, quadrant, xychart, etc.) to reduce code duplication.

// Allow dead code for utilities that are designed for future refactoring of other diagram types.
// As more diagrams are refactored to use these utilities, the warnings will naturally resolve.
#![allow(dead_code)]

use crate::render::svg::{Attrs, SvgElement, Theme};

// =============================================================================
// Common Chart Layout Constants
// =============================================================================

/// Default chart dimensions commonly used across chart types
pub mod dimensions {
    /// Default margin at top of chart
    pub const MARGIN_TOP: f64 = 50.0;
    /// Default margin at right of chart
    pub const MARGIN_RIGHT: f64 = 50.0;
    /// Default margin at bottom of chart
    pub const MARGIN_BOTTOM: f64 = 50.0;
    /// Default margin at left of chart
    pub const MARGIN_LEFT: f64 = 50.0;
    /// Default padding around chart elements
    pub const PADDING: f64 = 10.0;
    /// Default height reserved for title
    pub const TITLE_HEIGHT: f64 = 50.0;
    /// Default padding for axis labels
    pub const AXIS_LABEL_PADDING: f64 = 30.0;
}

/// Chart margins configuration
#[derive(Debug, Clone, Copy)]
pub struct ChartMargins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for ChartMargins {
    fn default() -> Self {
        Self {
            top: dimensions::MARGIN_TOP,
            right: dimensions::MARGIN_RIGHT,
            bottom: dimensions::MARGIN_BOTTOM,
            left: dimensions::MARGIN_LEFT,
        }
    }
}

impl ChartMargins {
    /// Create new margins with all sides equal
    pub fn uniform(margin: f64) -> Self {
        Self {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        }
    }

    /// Create new margins with vertical and horizontal values
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Total width added by margins
    pub fn horizontal_total(&self) -> f64 {
        self.left + self.right
    }

    /// Total height added by margins
    pub fn vertical_total(&self) -> f64 {
        self.top + self.bottom
    }
}

// =============================================================================
// Color Palette Utilities
// =============================================================================

/// Standard chart color palette matching mermaid.js defaults
pub const CHART_COLORS: &[&str] = &[
    "#4C78A8", // Blue
    "#F58518", // Orange
    "#E45756", // Red
    "#72B7B2", // Teal
    "#54A24B", // Green
    "#EECA3B", // Yellow
    "#B279A2", // Purple
    "#FF9DA6", // Pink
];

/// Get a color from the palette, cycling through if index exceeds palette size
pub fn get_chart_color(index: usize) -> &'static str {
    CHART_COLORS[index % CHART_COLORS.len()]
}

/// Get a color from a custom palette, cycling through if index exceeds palette size
pub fn get_color_from_palette<'a>(palette: &'a [String], index: usize) -> &'a str {
    &palette[index % palette.len()]
}

// =============================================================================
// Title Rendering
// =============================================================================

/// Configuration for title rendering
#[derive(Debug, Clone)]
pub struct TitleConfig {
    /// X position of the title
    pub x: f64,
    /// Y position of the title
    pub y: f64,
    /// Font size in pixels
    pub font_size: f64,
    /// CSS class name for styling
    pub class: String,
    /// Text anchor (start, middle, end)
    pub text_anchor: String,
}

impl Default for TitleConfig {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 25.0,
            font_size: 20.0,
            class: "chart-title".to_string(),
            text_anchor: "middle".to_string(),
        }
    }
}

impl TitleConfig {
    /// Create a centered title config
    pub fn centered(width: f64, y: f64) -> Self {
        Self {
            x: width / 2.0,
            y,
            ..Default::default()
        }
    }

    /// Set custom CSS class
    pub fn with_class(mut self, class: &str) -> Self {
        self.class = class.to_string();
        self
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
}

/// Render a chart title if present
///
/// Returns `Some(SvgElement)` if title is non-empty, `None` otherwise.
pub fn render_title(
    title: &str,
    config: &TitleConfig,
    fill_color: &str,
) -> Option<SvgElement> {
    if title.is_empty() {
        return None;
    }

    Some(SvgElement::Text {
        x: config.x,
        y: config.y,
        content: title.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", &config.text_anchor)
            .with_class(&config.class)
            .with_attr("font-size", &format!("{}", config.font_size))
            .with_attr("font-weight", "bold")
            .with_fill(fill_color),
    })
}

/// Calculate title offset (height to add when title is present)
pub fn title_offset(title: &str, height: f64) -> f64 {
    if title.is_empty() {
        0.0
    } else {
        height
    }
}

// =============================================================================
// Legend Rendering
// =============================================================================

/// A single legend item
#[derive(Debug, Clone)]
pub struct LegendItem {
    /// Label text
    pub label: String,
    /// Fill color for the legend box
    pub color: String,
    /// Optional extra text (e.g., value or percentage)
    pub extra: Option<String>,
}

impl LegendItem {
    /// Create a new legend item
    pub fn new(label: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: color.into(),
            extra: None,
        }
    }

    /// Add extra text (like value or percentage)
    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra = Some(extra.into());
        self
    }
}

/// Configuration for legend rendering
#[derive(Debug, Clone)]
pub struct LegendConfig {
    /// X position of the legend
    pub x: f64,
    /// Y position of the legend
    pub y: f64,
    /// Height of each legend item
    pub item_height: f64,
    /// Size of the color box
    pub box_size: f64,
    /// Gap between box and text
    pub text_gap: f64,
    /// Font size for labels
    pub font_size: f64,
    /// CSS class for legend group
    pub class: String,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            item_height: 22.0,
            box_size: 18.0,
            text_gap: 4.0,
            font_size: 17.0,
            class: "legend".to_string(),
        }
    }
}

impl LegendConfig {
    /// Create a new legend config at the specified position
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            ..Default::default()
        }
    }

    /// Set box size
    pub fn with_box_size(mut self, size: f64) -> Self {
        self.box_size = size;
        self
    }

    /// Set item height
    pub fn with_item_height(mut self, height: f64) -> Self {
        self.item_height = height;
        self
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
}

/// Render a legend from a list of items
///
/// Creates a group containing colored boxes and labels for each legend item.
/// Renders shapes first, then text for proper z-order.
pub fn render_legend(items: &[LegendItem], config: &LegendConfig) -> SvgElement {
    let mut children = Vec::new();

    // First pass: render all colored boxes (shapes before text for z-order)
    for (i, item) in items.iter().enumerate() {
        let item_y = config.y + (i as f64) * config.item_height;

        children.push(SvgElement::Rect {
            x: config.x,
            y: item_y,
            width: config.box_size,
            height: config.box_size,
            rx: None,
            ry: None,
            attrs: Attrs::new()
                .with_fill(&item.color)
                .with_stroke(&item.color)
                .with_class(&config.class),
        });
    }

    // Second pass: render all text labels
    for (i, item) in items.iter().enumerate() {
        let item_y = config.y + (i as f64) * config.item_height;
        let text_x = config.x + config.box_size + config.text_gap;
        let text_y = item_y + config.box_size * 0.78; // Approximately 14/18 for baseline alignment

        // Build label text with optional extra
        let label_text = if let Some(ref extra) = item.extra {
            format!("{} [{}]", item.label, extra)
        } else {
            item.label.clone()
        };

        children.push(SvgElement::Text {
            x: text_x,
            y: text_y,
            content: label_text,
            attrs: Attrs::new()
                .with_class(&config.class)
                .with_attr("font-size", &format!("{}", config.font_size)),
        });
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class(&config.class),
    }
}

// =============================================================================
// Data Range Calculations
// =============================================================================

/// Calculate the min and max values from an iterator of f64 values
pub fn calculate_range<'a>(values: impl Iterator<Item = &'a f64>) -> Option<(f64, f64)> {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    let mut has_values = false;

    for &value in values {
        has_values = true;
        min = min.min(value);
        max = max.max(value);
    }

    if has_values {
        Some((min, max))
    } else {
        None
    }
}

/// Calculate the data range with optional padding and zero inclusion
pub fn calculate_padded_range(
    values: impl Iterator<Item = f64>,
    padding_factor: f64,
    include_zero: bool,
) -> (f64, f64) {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    let mut has_values = false;

    for value in values {
        has_values = true;
        min = min.min(value);
        max = max.max(value);
    }

    if !has_values {
        return (0.0, 100.0); // Default range
    }

    // Calculate padding
    let range = max - min;
    let padding = if range > 0.0 {
        range * padding_factor
    } else {
        1.0
    };

    // Include zero if all positive or configured
    if include_zero && min >= 0.0 {
        min = 0.0;
    }

    (min - padding.min(min.abs() * padding_factor), max + padding)
}

// =============================================================================
// CSS Generation Helpers
// =============================================================================

/// Generate common chart CSS rules for a specific chart type
///
/// Returns CSS string with rules for background, title, and text elements.
pub fn generate_base_chart_css(
    theme: &Theme,
    prefix: &str,
) -> String {
    format!(
        r#"
.{prefix}-background {{
  fill: {background};
}}

.{prefix}-title {{
  fill: {text_color};
  font-family: {font_family};
}}

.{prefix}-text {{
  fill: {text_color};
  font-family: {font_family};
}}
"#,
        prefix = prefix,
        background = theme.background,
        text_color = theme.primary_text_color,
        font_family = theme.font_family,
    )
}

/// Generate axis CSS rules common to charts with axes
pub fn generate_axis_css(theme: &Theme, prefix: &str) -> String {
    format!(
        r#"
.{prefix}-axis {{
  stroke: {line_color};
  stroke-width: 1px;
}}

.{prefix}-axis-title {{
  fill: {text_color};
  font-family: {font_family};
}}

.{prefix}-axis-label {{
  fill: {text_color};
  font-family: {font_family};
}}

.{prefix}-tick {{
  stroke: {line_color};
  stroke-width: 1px;
}}
"#,
        prefix = prefix,
        line_color = theme.line_color,
        text_color = theme.primary_text_color,
        font_family = theme.font_family,
    )
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Format a number for display on axis labels
pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 || value.abs() >= 1000.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

/// Truncate a label if too long
pub fn truncate_label(label: &str, max_len: usize) -> String {
    if label.len() > max_len {
        format!("{}...", &label[..max_len - 3])
    } else {
        label.to_string()
    }
}

// Re-export escape_xml from svg module for convenience
pub use crate::render::svg::escape_xml;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_margins_default() {
        let margins = ChartMargins::default();
        assert_eq!(margins.top, dimensions::MARGIN_TOP);
        assert_eq!(margins.horizontal_total(), dimensions::MARGIN_LEFT + dimensions::MARGIN_RIGHT);
    }

    #[test]
    fn test_chart_margins_uniform() {
        let margins = ChartMargins::uniform(20.0);
        assert_eq!(margins.top, 20.0);
        assert_eq!(margins.right, 20.0);
        assert_eq!(margins.bottom, 20.0);
        assert_eq!(margins.left, 20.0);
    }

    #[test]
    fn test_get_chart_color() {
        assert_eq!(get_chart_color(0), "#4C78A8");
        assert_eq!(get_chart_color(8), "#4C78A8"); // Should cycle
    }

    #[test]
    fn test_title_offset() {
        assert_eq!(title_offset("", 50.0), 0.0);
        assert_eq!(title_offset("My Title", 50.0), 50.0);
    }

    #[test]
    fn test_legend_item() {
        let item = LegendItem::new("Test", "#FF0000")
            .with_extra("42%");
        assert_eq!(item.label, "Test");
        assert_eq!(item.color, "#FF0000");
        assert_eq!(item.extra, Some("42%".to_string()));
    }

    #[test]
    fn test_calculate_range() {
        let values = vec![1.0, 5.0, 3.0, 2.0];
        let (min, max) = calculate_range(values.iter()).unwrap();
        assert_eq!(min, 1.0);
        assert_eq!(max, 5.0);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(100.0), "100");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(1234.0), "1234");
    }

    #[test]
    fn test_truncate_label() {
        assert_eq!(truncate_label("short", 10), "short");
        assert_eq!(truncate_label("this is a very long label", 10), "this is...");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
