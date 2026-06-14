//! Size estimation for layout

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use super::adapter::{NodeSizeConfig, SizeEstimator};
use super::types::NodeShape;

/// Character-width based size estimator
///
/// This estimator uses average character widths to approximate text dimensions
/// without requiring a rendering context. It's suitable for layout purposes
/// where exact pixel-perfect sizing isn't critical.
#[derive(Debug, Clone)]
pub struct CharacterSizeEstimator {
    /// Average character width ratio (relative to font size)
    pub char_width_ratio: f64,
    /// Line height ratio (relative to font size)
    pub line_height_ratio: f64,
}

impl Default for CharacterSizeEstimator {
    fn default() -> Self {
        Self {
            // Approximate ratio for proportional fonts like trebuchet ms
            // Calibrated to match mermaid.js foreignObject text rendering
            // Mermaid.js uses actual browser getBBox which varies by font/platform
            char_width_ratio: 0.6,
            // HTML text in foreignObject has ~2.3x line-height due to
            // default line-height:1.5 plus <p> element margins
            line_height_ratio: 2.3,
        }
    }
}

impl CharacterSizeEstimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an estimator optimized for monospace fonts
    pub fn monospace() -> Self {
        Self {
            char_width_ratio: 0.6,
            line_height_ratio: 1.2,
        }
    }
}

impl SizeEstimator for CharacterSizeEstimator {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        if text.is_empty() {
            return (0.0, font_size * self.line_height_ratio);
        }

        // Normalize <br> variants to newlines for proper line counting
        let normalized = crate::render::text_utils::normalize_br_tags(text);

        let lines: Vec<&str> = normalized.lines().collect();
        let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let num_lines = lines.len().max(1);

        let width = (max_chars as f64) * font_size * self.char_width_ratio;
        let height = (num_lines as f64) * font_size * self.line_height_ratio;

        (width, height)
    }

    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        // Calculate text dimensions
        let (text_width, text_height) = label
            .map(|l| self.estimate_text_size(l, config.font_size))
            .unwrap_or((0.0, 0.0));

        // Add padding
        let base_width = text_width + config.padding_horizontal * 2.0;
        let base_height = text_height + config.padding_vertical * 2.0;

        // Apply shape-specific adjustments
        let (width, height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle => {
                // Circle sizing per mermaid.js circle.ts:
                // radius = bbox.width / 2 + halfPadding (where halfPadding = node.padding / 2 = 4)
                // diameter = bbox.width + padding (effectively text_width + 8)
                let half_padding = config.padding_vertical / 2.0; // 4, matches mermaid's halfPadding
                let diameter = text_width.max(text_height) + half_padding * 2.0;
                (diameter, diameter)
            }
            NodeShape::Diamond => {
                // Diamond is a square rotated 45 degrees, matching mermaid.js question.ts:
                // mermaid.js uses single padding (node.padding = 8) for diamonds, not double
                // w = bbox.width + padding, h = bbox.height + padding, s = w + h
                let single_padding = config.padding_vertical; // 8, matches mermaid's node.padding
                let w = text_width + single_padding;
                let h = text_height + single_padding;
                let s = w + h;
                (s, s)
            }
            NodeShape::Hexagon => {
                // Hexagon needs extra horizontal space for angled sides
                (base_width * 1.2, base_height)
            }
            NodeShape::Ellipse => {
                // Ellipse needs slightly more space
                (base_width * 1.1, base_height * 1.1)
            }
            NodeShape::Stadium => {
                // Stadium (pill shape) needs extra width for rounded ends
                (base_width + base_height, base_height)
            }
            NodeShape::Cylinder => {
                // Cylinder needs extra height for 3D cap
                (base_width, base_height * 1.3)
            }
            NodeShape::Trapezoid | NodeShape::InvTrapezoid => {
                // Trapezoid needs extra width for angled sides
                (base_width * 1.2, base_height)
            }
            NodeShape::LeanRight | NodeShape::LeanLeft => {
                // Parallelogram needs extra width
                (base_width * 1.2, base_height)
            }
            NodeShape::Subroutine => {
                // Subroutine has extra side bars
                (base_width + 20.0, base_height)
            }
            NodeShape::Odd => {
                // Odd shape (flag-like) - asymmetric
                (base_width * 1.1, base_height)
            }
            NodeShape::HorizontalBar => {
                // Fork/join bar: fixed dimensions, ignore text
                (70.0, 10.0)
            }
            NodeShape::Rectangle | NodeShape::RoundedRect => {
                // Standard rectangles - no adjustment needed
                (base_width, base_height)
            }
        };

        // Apply min/max constraints
        // For shapes that must be square (circle, diamond), use max of both constraints
        let (final_width, final_height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Diamond => {
                let min_dim = config.min_width.max(config.min_height);
                let dim = width.max(min_dim);
                (dim, dim)
            }
            _ => {
                let w = width.max(config.min_width);
                let h = height.max(config.min_height);
                (w, h)
            }
        };
        let final_width = config
            .max_width
            .map(|max| final_width.min(max))
            .unwrap_or(final_width);

        (final_width, final_height)
    }
}

/// Size estimator wrapper that memoizes repeated text and node measurements.
///
/// Mermaid's browser renderer memoizes text dimension work because repeated
/// labels are common in generated or large diagrams. This wrapper provides the
/// same behavior for Rust-side layout while exposing counters that benchmarks
/// can assert against.
#[derive(Debug, Clone)]
pub struct CachedSizeEstimator<E> {
    inner: E,
    text_cache: RefCell<HashMap<TextSizeKey, (f64, f64)>>,
    node_cache: RefCell<HashMap<NodeSizeKey, (f64, f64)>>,
    text_cache_hits: Cell<usize>,
    text_cache_misses: Cell<usize>,
    node_cache_hits: Cell<usize>,
    node_cache_misses: Cell<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeasurementStats {
    pub text_cache_hits: usize,
    pub text_cache_misses: usize,
    pub text_measurements: usize,
    pub node_cache_hits: usize,
    pub node_cache_misses: usize,
    pub node_measurements: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextSizeKey {
    text: String,
    font_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NodeSizeKey {
    label: Option<String>,
    shape: NodeShape,
    config: NodeSizeConfigKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NodeSizeConfigKey {
    font_size: u64,
    padding_horizontal: u64,
    padding_vertical: u64,
    min_width: u64,
    min_height: u64,
    max_width: Option<u64>,
}

impl From<&NodeSizeConfig> for NodeSizeConfigKey {
    fn from(config: &NodeSizeConfig) -> Self {
        Self {
            font_size: config.font_size.to_bits(),
            padding_horizontal: config.padding_horizontal.to_bits(),
            padding_vertical: config.padding_vertical.to_bits(),
            min_width: config.min_width.to_bits(),
            min_height: config.min_height.to_bits(),
            max_width: config.max_width.map(f64::to_bits),
        }
    }
}

impl<E> CachedSizeEstimator<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            text_cache: RefCell::new(HashMap::new()),
            node_cache: RefCell::new(HashMap::new()),
            text_cache_hits: Cell::new(0),
            text_cache_misses: Cell::new(0),
            node_cache_hits: Cell::new(0),
            node_cache_misses: Cell::new(0),
        }
    }

    pub fn stats(&self) -> MeasurementStats {
        let text_cache_misses = self.text_cache_misses.get();
        let node_cache_misses = self.node_cache_misses.get();
        MeasurementStats {
            text_cache_hits: self.text_cache_hits.get(),
            text_cache_misses,
            text_measurements: text_cache_misses,
            node_cache_hits: self.node_cache_hits.get(),
            node_cache_misses,
            node_measurements: node_cache_misses,
        }
    }
}

impl<E: SizeEstimator> SizeEstimator for CachedSizeEstimator<E> {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        let key = TextSizeKey {
            text: text.to_string(),
            font_size: font_size.to_bits(),
        };

        if let Some(size) = self.text_cache.borrow().get(&key).copied() {
            self.text_cache_hits.set(self.text_cache_hits.get() + 1);
            return size;
        }

        self.text_cache_misses.set(self.text_cache_misses.get() + 1);
        let size = self.inner.estimate_text_size(text, font_size);
        self.text_cache.borrow_mut().insert(key, size);
        size
    }

    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        let key = NodeSizeKey {
            label: label.map(str::to_string),
            shape,
            config: config.into(),
        };

        if let Some(size) = self.node_cache.borrow().get(&key).copied() {
            self.node_cache_hits.set(self.node_cache_hits.get() + 1);
            return size;
        }

        self.node_cache_misses.set(self.node_cache_misses.get() + 1);
        let size = self.inner.estimate_node_size(label, shape, config);
        self.node_cache.borrow_mut().insert(key, size);
        size
    }
}

/// Font-based size estimator using fontdue for accurate text measurement
///
/// This estimator uses actual font metrics to calculate text dimensions,
/// matching browser getBBox() behavior for better visual parity with mermaid.js.
#[derive(Debug)]
pub struct FontdueSizeEstimator {
    /// The loaded font for text measurement
    font: fontdue::Font,
    /// Line height ratio (relative to font size)
    line_height_ratio: f64,
}

impl FontdueSizeEstimator {
    /// Create a new estimator from font data
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        Ok(Self {
            font,
            line_height_ratio: 1.5, // Standard HTML line-height
        })
    }

    /// Try to create an estimator by loading a system font
    ///
    /// Attempts to find and load fonts in this order:
    /// 1. DejaVu Sans (common on Linux)
    /// 2. Arial (common on Windows/Mac)
    /// 3. Helvetica (common on Mac)
    pub fn try_system_font() -> Option<Self> {
        // Common font paths by platform
        let font_paths = [
            // Linux
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            // Mac
            "/System/Library/Fonts/Helvetica.ttc",
            "/Library/Fonts/Arial.ttf",
            // Windows
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\verdana.ttf",
        ];

        for path in font_paths {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(estimator) = Self::from_bytes(&data) {
                    return Some(estimator);
                }
            }
        }
        None
    }

    /// Measure text width using font metrics
    fn measure_text_width(&self, text: &str, font_size: f64) -> f64 {
        let px = font_size as f32;
        text.chars()
            .map(|c| {
                let metrics = self.font.metrics(c, px);
                metrics.advance_width as f64
            })
            .sum()
    }
}

impl SizeEstimator for FontdueSizeEstimator {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        if text.is_empty() {
            return (0.0, font_size * self.line_height_ratio);
        }

        // Normalize <br> variants to newlines for proper line counting
        let normalized = crate::render::text_utils::normalize_br_tags(text);

        let lines: Vec<&str> = normalized.lines().collect();
        let num_lines = lines.len().max(1);

        // Measure actual width of each line using font metrics
        let width = lines
            .iter()
            .map(|line| self.measure_text_width(line, font_size))
            .fold(0.0_f64, |max, w| max.max(w));

        let height = (num_lines as f64) * font_size * self.line_height_ratio;

        (width, height)
    }

    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        // Calculate text dimensions using font metrics
        let (text_width, text_height) = label
            .map(|l| self.estimate_text_size(l, config.font_size))
            .unwrap_or((0.0, 0.0));

        // Add padding
        let base_width = text_width + config.padding_horizontal * 2.0;
        let base_height = text_height + config.padding_vertical * 2.0;

        // Apply shape-specific adjustments (same as CharacterSizeEstimator)
        let (width, height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle => {
                let half_padding = config.padding_vertical / 2.0;
                let diameter = text_width.max(text_height) + half_padding * 2.0;
                (diameter, diameter)
            }
            NodeShape::Diamond => {
                let single_padding = config.padding_vertical;
                let w = text_width + single_padding;
                let h = text_height + single_padding;
                let s = w + h;
                (s, s)
            }
            NodeShape::Hexagon => (base_width * 1.2, base_height),
            NodeShape::Ellipse => (base_width * 1.1, base_height * 1.1),
            NodeShape::Stadium => (base_width + base_height, base_height),
            NodeShape::Cylinder => (base_width, base_height * 1.3),
            NodeShape::Trapezoid | NodeShape::InvTrapezoid => (base_width * 1.2, base_height),
            NodeShape::LeanRight | NodeShape::LeanLeft => (base_width * 1.2, base_height),
            NodeShape::Subroutine => (base_width + 20.0, base_height),
            NodeShape::Odd => (base_width * 1.1, base_height),
            NodeShape::HorizontalBar => (70.0, 10.0),
            NodeShape::Rectangle | NodeShape::RoundedRect => (base_width, base_height),
        };

        // Apply min/max constraints
        let (final_width, final_height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Diamond => {
                let min_dim = config.min_width.max(config.min_height);
                let dim = width.max(min_dim);
                (dim, dim)
            }
            _ => {
                let w = width.max(config.min_width);
                let h = height.max(config.min_height);
                (w, h)
            }
        };
        let final_width = config
            .max_width
            .map(|max| final_width.min(max))
            .unwrap_or(final_width);

        (final_width, final_height)
    }
}

/// Create the best available size estimator
///
/// Tries to load a system font for accurate measurements.
/// Falls back to character-based estimation if no font is available.
pub fn create_size_estimator() -> Box<dyn SizeEstimator> {
    if let Some(font_estimator) = FontdueSizeEstimator::try_system_font() {
        Box::new(font_estimator)
    } else {
        Box::new(CharacterSizeEstimator::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_size_estimation() {
        let estimator = CharacterSizeEstimator::default();

        let (w, h) = estimator.estimate_text_size("Hello", 14.0);
        assert!(w > 0.0);
        assert!(h > 0.0);

        // Longer text should be wider
        let (w2, _) = estimator.estimate_text_size("Hello World", 14.0);
        assert!(w2 > w);

        // Multiline text should be taller
        let (_, h2) = estimator.estimate_text_size("Line1\nLine2", 14.0);
        assert!(h2 > h);
    }

    #[test]
    fn test_node_size_with_shapes() {
        let estimator = CharacterSizeEstimator::default();
        let config = NodeSizeConfig::default();

        let (rect_w, rect_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Rectangle, &config);

        // Diamond should be larger than rectangle for same text
        let (diamond_w, diamond_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Diamond, &config);
        assert!(diamond_w > rect_w);
        assert!(diamond_h > rect_h);

        // Circle should have equal width and height
        let (circle_w, circle_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Circle, &config);
        assert!((circle_w - circle_h).abs() < 0.001);
    }

    #[test]
    fn cached_estimator_reuses_repeated_node_measurements() {
        let estimator = CachedSizeEstimator::new(CharacterSizeEstimator::default());
        let config = NodeSizeConfig::default();

        for _ in 0..10 {
            let (width, height) =
                estimator.estimate_node_size(Some("Repeated"), NodeShape::Rectangle, &config);
            assert!(width > 0.0);
            assert!(height > 0.0);
        }

        let stats = estimator.stats();
        assert_eq!(stats.node_cache_misses, 1);
        assert_eq!(stats.node_cache_hits, 9);
        assert!(
            stats.node_measurements < 10,
            "repeated labels should avoid one measurement per node"
        );
    }

    #[test]
    fn test_min_size_constraints() {
        let estimator = CharacterSizeEstimator::default();
        let config = NodeSizeConfig {
            min_width: 100.0,
            min_height: 50.0,
            ..Default::default()
        };

        // Even with no label, should meet minimum size
        let (w, h) = estimator.estimate_node_size(None, NodeShape::Rectangle, &config);
        assert!(w >= 100.0);
        assert!(h >= 50.0);
    }
}
