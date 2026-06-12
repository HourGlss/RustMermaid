//! ASCII renderer for radar/spider chart diagrams.
//!
//! Renders radar charts as a true radial shape using braille characters
//! for the chart body (graticule, axes, curves), with text labels placed
//! around the perimeter and a legend below.

use std::f64::consts::PI;

use crate::diagrams::radar::{Graticule, RadarAxis, RadarCurve, RadarDb, RadarOptions};
use crate::error::Result;

use super::canvas::BrailleCanvas;

/// Chart radius in character cell columns.
const CHART_CELL_RADIUS: usize = 12;

/// How far outside the chart perimeter to place axis labels
/// (as a fraction of the radius in cell coordinates).
const LABEL_FACTOR: f64 = 1.15;

/// Number of line segments used to approximate a circle.
const CIRCLE_SEGMENTS: usize = 64;

/// Markers to visually differentiate curves in monochrome output.
const CURVE_MARKERS: &[char] = &['●', '◆', '■', '▲', '★', '◉', '▶', '◈'];

/// Render a radar chart as character art with a radial layout.
pub fn render_radar_ascii(db: &RadarDb) -> Result<String> {
    let axes = db.get_axes();
    let curves = db.get_curves();
    let options = db.get_options();

    if axes.is_empty() || curves.is_empty() {
        let title = db.get_title();
        return Ok(empty_radar_output(title));
    }

    let n = axes.len();
    let max_val = radar_max_value(options, curves);
    let geometry = RadarGeometry::new(n);
    let mut canvas = BrailleCanvas::new(geometry.chart_cols, geometry.chart_rows);

    draw_graticule(&mut canvas, options, &geometry);
    draw_spokes(&mut canvas, &geometry);
    let curve_points = compute_curve_points(curves, options.min, max_val, &geometry);
    draw_curve_polygons(&mut canvas, &curve_points);

    let axis_labels = axis_labels(axes);
    let buffer = RadarBuffer::new(&axis_labels, &geometry);
    let mut buf = vec![vec![' '; buffer.cols]; buffer.rows];
    place_braille_grid(&mut buf, &buffer, canvas.to_char_grid());
    place_axis_labels(&mut buf, &buffer, &geometry, &axis_labels);
    place_curve_markers(&mut buf, &buffer, &curve_points);

    Ok(build_radar_output(db, options, curves, &buf))
}

fn empty_radar_output(title: &str) -> String {
    if title.is_empty() {
        "(empty radar chart)\n".to_string()
    } else {
        format!("{}\n\n(empty radar chart)\n", title)
    }
}

fn radar_max_value(options: &RadarOptions, curves: &[RadarCurve]) -> f64 {
    options.max.unwrap_or_else(|| {
        curves
            .iter()
            .flat_map(|c| c.entries.iter().copied())
            .fold(0.0f64, f64::max)
    })
}

struct RadarGeometry {
    angles: Vec<f64>,
    chart_cols: usize,
    chart_rows: usize,
    px_cx: f64,
    px_cy: f64,
    px_r: f64,
}

impl RadarGeometry {
    fn new(axis_count: usize) -> Self {
        let chart_cols = 2 * CHART_CELL_RADIUS + 1;
        let chart_rows = CHART_CELL_RADIUS + 1;
        let canvas = BrailleCanvas::new(chart_cols, chart_rows);
        Self {
            angles: (0..axis_count)
                .map(|i| -PI / 2.0 + (i as f64) * 2.0 * PI / (axis_count as f64))
                .collect(),
            chart_cols,
            chart_rows,
            px_cx: canvas.pixel_width() as f64 / 2.0,
            px_cy: canvas.pixel_height() as f64 / 2.0,
            px_r: (CHART_CELL_RADIUS * 2) as f64,
        }
    }
}

struct RadarBuffer {
    cols: usize,
    rows: usize,
    margin_x: usize,
    margin_top: usize,
    center_col: usize,
    center_row: usize,
}

impl RadarBuffer {
    fn new(axis_labels: &[&str], geometry: &RadarGeometry) -> Self {
        let max_label_len = axis_labels
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let margin_x = max_label_len + 3;
        let margin_top = 2;
        let margin_bottom = 2;
        let cols = margin_x + geometry.chart_cols + margin_x;
        let rows = margin_top + geometry.chart_rows + margin_bottom;

        Self {
            cols,
            rows,
            margin_x,
            margin_top,
            center_col: margin_x + geometry.chart_cols / 2,
            center_row: margin_top + geometry.chart_rows / 2,
        }
    }
}

fn draw_graticule(canvas: &mut BrailleCanvas, options: &RadarOptions, geometry: &RadarGeometry) {
    for t in 1..=options.ticks {
        let frac = t as f64 / options.ticks as f64;
        let ring_r = geometry.px_r * frac;
        match options.graticule {
            Graticule::Circle => draw_circle(canvas, geometry.px_cx, geometry.px_cy, ring_r),
            Graticule::Polygon => draw_polygon(
                canvas,
                geometry.px_cx,
                geometry.px_cy,
                ring_r,
                &geometry.angles,
            ),
        }
    }
}

fn draw_spokes(canvas: &mut BrailleCanvas, geometry: &RadarGeometry) {
    for angle in &geometry.angles {
        let ex = geometry.px_cx + geometry.px_r * angle.cos();
        let ey = geometry.px_cy + geometry.px_r * angle.sin();
        canvas.draw_line(
            geometry.px_cx as isize,
            geometry.px_cy as isize,
            ex as isize,
            ey as isize,
        );
    }
}

fn compute_curve_points(
    curves: &[RadarCurve],
    min_val: f64,
    max_val: f64,
    geometry: &RadarGeometry,
) -> Vec<Vec<(f64, f64)>> {
    let n = geometry.angles.len();
    curves
        .iter()
        .map(|curve| {
            (0..n)
                .map(|i| {
                    let val = curve.entries.get(i).copied().unwrap_or(0.0);
                    let frac = relative_radius(val, min_val, max_val);
                    let x = geometry.px_cx + geometry.px_r * frac * geometry.angles[i].cos();
                    let y = geometry.px_cy + geometry.px_r * frac * geometry.angles[i].sin();
                    (x, y)
                })
                .collect()
        })
        .collect()
}

fn draw_curve_polygons(canvas: &mut BrailleCanvas, curve_points: &[Vec<(f64, f64)>]) {
    for points in curve_points {
        let len = points.len();
        for j in 0..len {
            let k = (j + 1) % len;
            canvas.draw_line(
                points[j].0 as isize,
                points[j].1 as isize,
                points[k].0 as isize,
                points[k].1 as isize,
            );
        }
    }
}

fn axis_labels(axes: &[RadarAxis]) -> Vec<&str> {
    axes.iter()
        .map(|a| {
            if !a.label.is_empty() {
                a.label.as_str()
            } else {
                a.name.as_str()
            }
        })
        .collect()
}

fn place_braille_grid(buf: &mut [Vec<char>], buffer: &RadarBuffer, braille_grid: Vec<Vec<char>>) {
    for (row, braille_row) in braille_grid.iter().enumerate() {
        for (col, &ch) in braille_row.iter().enumerate() {
            let br = buffer.margin_top + row;
            let bc = buffer.margin_x + col;
            if br < buffer.rows && bc < buffer.cols {
                buf[br][bc] = ch;
            }
        }
    }
}

fn place_axis_labels(
    buf: &mut [Vec<char>],
    buffer: &RadarBuffer,
    geometry: &RadarGeometry,
    axis_labels: &[&str],
) {
    let cell_r_x = CHART_CELL_RADIUS as f64;
    let cell_r_y = CHART_CELL_RADIUS as f64 / 2.0;

    for (i, label) in axis_labels.iter().enumerate() {
        let theta = geometry.angles[i];
        let lx = buffer.center_col as f64 + LABEL_FACTOR * cell_r_x * theta.cos();
        let ly = buffer.center_row as f64 + LABEL_FACTOR * cell_r_y * theta.sin();

        let label_len = label.chars().count();
        let start_col = label_start_col(theta.cos(), lx, label_len);
        let row = ly.round().max(0.0) as usize;

        for (j, ch) in label.chars().enumerate() {
            let col = start_col + j as isize;
            if col >= 0 && (col as usize) < buffer.cols && row < buffer.rows {
                buf[row][col as usize] = ch;
            }
        }
    }
}

fn label_start_col(cos_t: f64, lx: f64, label_len: usize) -> isize {
    if cos_t > 0.3 {
        lx.round() as isize + 1
    } else if cos_t < -0.3 {
        lx.round() as isize - label_len as isize
    } else {
        lx.round() as isize - (label_len as isize) / 2
    }
}

fn place_curve_markers(
    buf: &mut [Vec<char>],
    buffer: &RadarBuffer,
    curve_points: &[Vec<(f64, f64)>],
) {
    for (ci, points) in curve_points.iter().enumerate() {
        let marker = CURVE_MARKERS[ci % CURVE_MARKERS.len()];
        for &(px, py) in points {
            let col = buffer.margin_x + (px.round() as usize / 2);
            let row = buffer.margin_top + (py.round() as usize / 4);
            if row < buffer.rows && col < buffer.cols {
                buf[row][col] = marker;
            }
        }
    }
}

fn build_radar_output(
    db: &RadarDb,
    options: &RadarOptions,
    curves: &[RadarCurve],
    buf: &[Vec<char>],
) -> String {
    let mut lines: Vec<String> = Vec::new();

    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push("─".repeat(title.chars().count().max(40)));
        lines.push(String::new());
    }

    // Chart
    for row in buf {
        let line: String = row.iter().collect();
        lines.push(line.trim_end().to_string());
    }

    // Legend
    if options.show_legend && !curves.is_empty() {
        lines.push(String::new());
        let legend_parts: Vec<String> = curves
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let marker = CURVE_MARKERS[i % CURVE_MARKERS.len()];
                let label = if !c.label.is_empty() {
                    &c.label
                } else {
                    &c.name
                };
                format!("{} {}", marker, label)
            })
            .collect();
        lines.push(format!("  Legend: {}", legend_parts.join("  ")));
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Calculate relative radius (0.0 to 1.0) for a value within min/max range.
fn relative_radius(value: f64, min_value: f64, max_value: f64) -> f64 {
    let clipped = value.clamp(min_value, max_value);
    if (max_value - min_value).abs() < f64::EPSILON {
        return 1.0;
    }
    (clipped - min_value) / (max_value - min_value)
}

/// Draw a circle approximated by line segments on the braille canvas.
fn draw_circle(canvas: &mut BrailleCanvas, cx: f64, cy: f64, r: f64) {
    for i in 0..CIRCLE_SEGMENTS {
        let t0 = 2.0 * PI * i as f64 / CIRCLE_SEGMENTS as f64;
        let t1 = 2.0 * PI * (i + 1) as f64 / CIRCLE_SEGMENTS as f64;
        canvas.draw_line(
            (cx + r * t0.cos()) as isize,
            (cy + r * t0.sin()) as isize,
            (cx + r * t1.cos()) as isize,
            (cy + r * t1.sin()) as isize,
        );
    }
}

/// Draw a polygon connecting axis positions at a given radius.
fn draw_polygon(canvas: &mut BrailleCanvas, cx: f64, cy: f64, r: f64, angles: &[f64]) {
    let n = angles.len();
    for i in 0..n {
        let j = (i + 1) % n;
        canvas.draw_line(
            (cx + r * angles[i].cos()) as isize,
            (cy + r * angles[i].sin()) as isize,
            (cx + r * angles[j].cos()) as isize,
            (cy + r * angles[j].sin()) as isize,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_radar() {
        let db = RadarDb::new();
        let output = render_radar_ascii(&db).unwrap();
        assert!(output.contains("empty radar"));
    }

    #[test]
    fn renders_radial_shape_not_bars() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();

        // Should use braille characters for radar shape (radial layout)
        let has_braille = output
            .chars()
            .any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        assert!(
            has_braille,
            "Radar chart should use braille characters for radial shape\nOutput:\n{}",
            output
        );

        // Should NOT render as a bar chart
        assert!(
            !output.contains('█'),
            "Should not render as bar chart\nOutput:\n{}",
            output
        );
        assert!(
            !output.contains('░'),
            "Should not render as bar chart\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn gallery_radar_renders() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(output.contains("Skills Assessment"), "Output:\n{}", output);
        assert!(output.contains("Coding"), "Output:\n{}", output);
        assert!(output.contains("Testing"), "Output:\n{}", output);
        assert!(output.contains("Design"), "Output:\n{}", output);
        assert!(output.contains("Code Review"), "Output:\n{}", output);
        assert!(output.contains("Documentation"), "Output:\n{}", output);
    }

    #[test]
    fn curves_appear_in_legend() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains("Team Alpha"),
            "Should show curve name\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Team Beta"),
            "Should show curve name\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Legend"),
            "Should have legend section\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn has_data_point_markers() {
        let input = std::fs::read_to_string("docs/sources/radar.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains('●'),
            "Should have data point markers for first curve\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('◆'),
            "Should have data point markers for second curve\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn complex_radar_renders() {
        let input = std::fs::read_to_string("docs/sources/radar_complex.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Radar(db) => db,
            _ => panic!("Expected radar"),
        };
        let output = render_radar_ascii(&db).unwrap();
        assert!(
            output.contains("Programming Language Comparison"),
            "Should show title\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Performance"),
            "Should have axis label\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Rust"),
            "Should show curve\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("Python"),
            "Should show curve\nOutput:\n{}",
            output
        );
    }
}
