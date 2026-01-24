//! Shared utilities for chart-type diagram renderers
//!
//! This module provides common functionality used across multiple chart types
//! (pie, radar, quadrant, xychart, etc.) to reduce code duplication.

use crate::render::svg::{Attrs, SvgElement};

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

/// Render a chart title if present
///
/// Returns `Some(SvgElement)` if title is non-empty, `None` otherwise.
pub fn render_title(title: &str, config: &TitleConfig, fill_color: &str) -> Option<SvgElement> {
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
// Text Wrapping Utilities
// =============================================================================

/// Wrap text into lines based on a maximum character count per line.
///
/// Words are kept intact; a line break occurs when adding the next word
/// would exceed `max_chars`. Useful for fixed-width contexts.
///
/// # Example
/// ```ignore
/// use selkie::render::chart_utils::wrap_text_by_chars;
/// let lines = wrap_text_by_chars("hello world foo", 10);
/// assert_eq!(lines, vec!["hello", "world foo"]);
/// ```
pub fn wrap_text_by_chars(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Wrap text into lines based on estimated pixel width.
///
/// Uses a simple character width estimation (average character width = font_size * 0.55).
/// For more accurate wrapping, use [`wrap_text_by_width_fn`] with a custom estimator.
pub fn wrap_text_by_width(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    wrap_text_by_width_fn(text, max_width, |s| {
        estimate_text_width_simple(s, font_size)
    })
}

/// Wrap text into lines using a custom width estimation function.
///
/// This is the most flexible text wrapping function, allowing callers to provide
/// their own width estimation logic for different font metrics.
pub fn wrap_text_by_width_fn<F>(text: &str, max_width: f64, estimate_width: F) -> Vec<String>
where
    F: Fn(&str) -> f64,
{
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else {
            let potential = format!("{} {}", current_line, word);
            if estimate_width(&potential) <= max_width {
                current_line = potential;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Estimate text width using a simple average character width.
///
/// Assumes monospace-like behavior where each character is approximately
/// `font_size * 0.55` pixels wide. This is a reasonable approximation for
/// most proportional fonts when precise measurements aren't needed.
/// Normalize HTML `<br>` tags to newlines for consistent text processing.
///
/// Converts all common `<br>` variants (`<br>`, `<br/>`, `<br />`) to `\n`.
/// This is useful for text wrapping and line counting across diagram renderers.
#[inline]
pub fn normalize_br_tags(text: &str) -> String {
    text.replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
}

#[inline]
pub fn estimate_text_width_simple(text: &str, font_size: f64) -> f64 {
    text.chars().count() as f64 * font_size * 0.55
}

/// Estimate text height based on line count and line height.
#[inline]
pub fn estimate_text_height(line_count: usize, line_height: f64) -> f64 {
    line_count as f64 * line_height
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

// =============================================================================
// Background Rendering
// =============================================================================

/// Create a background rectangle element for a chart
///
/// Returns an SvgElement::Rect that fills the entire chart area with the
/// specified background color and CSS class.
pub fn render_background(width: f64, height: f64, fill: &str, class: &str) -> SvgElement {
    SvgElement::Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
        rx: None,
        ry: None,
        attrs: Attrs::new().with_fill(fill).with_class(class),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_margins_default() {
        let margins = ChartMargins::default();
        assert_eq!(margins.top, dimensions::MARGIN_TOP);
        assert_eq!(
            margins.horizontal_total(),
            dimensions::MARGIN_LEFT + dimensions::MARGIN_RIGHT
        );
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
        let item = LegendItem::new("Test", "#FF0000").with_extra("42%");
        assert_eq!(item.label, "Test");
        assert_eq!(item.color, "#FF0000");
        assert_eq!(item.extra, Some("42%".to_string()));
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
        assert_eq!(
            truncate_label("this is a very long label", 10),
            "this is..."
        );
    }

    #[test]
    fn test_wrap_text_by_chars() {
        // Basic wrapping
        let lines = wrap_text_by_chars("hello world foo bar", 12);
        assert_eq!(lines, vec!["hello world", "foo bar"]);

        // Single word longer than max
        let lines = wrap_text_by_chars("superlongword short", 10);
        assert_eq!(lines, vec!["superlongword", "short"]);

        // Empty text
        let lines = wrap_text_by_chars("", 10);
        assert_eq!(lines, vec![""]);

        // Fits in one line
        let lines = wrap_text_by_chars("short", 20);
        assert_eq!(lines, vec!["short"]);
    }

    #[test]
    fn test_wrap_text_by_width() {
        // With font_size=14, char width ~7.7px, so "hello world" ~85px
        let lines = wrap_text_by_width("hello world foo", 90.0, 14.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello world");

        // Empty text
        let lines = wrap_text_by_width("", 100.0, 14.0);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn test_estimate_text_width_simple() {
        // 5 chars * 14 * 0.55 = 38.5
        let width = estimate_text_width_simple("hello", 14.0);
        assert!((width - 38.5).abs() < 0.01);
    }

    #[test]
    fn test_estimate_text_height() {
        assert_eq!(estimate_text_height(3, 18.0), 54.0);
        assert_eq!(estimate_text_height(0, 18.0), 0.0);
    }

    #[test]
    fn test_normalize_br_tags() {
        // All variants should be converted to newlines
        assert_eq!(normalize_br_tags("hello<br>world"), "hello\nworld");
        assert_eq!(normalize_br_tags("hello<br/>world"), "hello\nworld");
        assert_eq!(normalize_br_tags("hello<br />world"), "hello\nworld");

        // Multiple br tags
        assert_eq!(normalize_br_tags("a<br>b<br/>c<br />d"), "a\nb\nc\nd");

        // No br tags
        assert_eq!(normalize_br_tags("hello world"), "hello world");

        // Empty string
        assert_eq!(normalize_br_tags(""), "");
    }
}
