use regex::Regex;
use selkie::{parse, render};

const SPACE_AGE_PROGRESSION: &str = "docs/sources/space_age_progression.mmd";
const MAX_INTRINSIC_DIMENSION: f64 = 16_000.0;

#[test]
fn space_age_progression_svg_intrinsic_size_is_capped() {
    let input = std::fs::read_to_string(SPACE_AGE_PROGRESSION)
        .expect("failed to read space_age_progression.mmd");

    let diagram = parse(&input).expect("failed to parse space_age_progression.mmd");
    let svg = render(&diagram).expect("failed to render space_age_progression.mmd");

    let width = svg_number_attr(&svg, "width");
    let height = svg_number_attr(&svg, "height");
    assert!(
        width <= MAX_INTRINSIC_DIMENSION,
        "SVG width should be capped at {MAX_INTRINSIC_DIMENSION}px, got {width}"
    );
    assert!(
        height <= MAX_INTRINSIC_DIMENSION,
        "SVG height should be capped at {MAX_INTRINSIC_DIMENSION}px, got {height}"
    );

    let (_, _, view_box_width, view_box_height) = svg_view_box(&svg);
    assert!(
        view_box_width > width || view_box_height > height,
        "large graph viewBox should preserve the full layout even when intrinsic size is capped"
    );
    assert!(
        view_box_width > MAX_INTRINSIC_DIMENSION || view_box_height > MAX_INTRINSIC_DIMENSION,
        "fixture should remain large enough to exercise invalid-size regressions"
    );
}

fn svg_number_attr(svg: &str, attr: &str) -> f64 {
    let pattern = Regex::new(&format!(r#"{attr}="([^"]+)""#)).unwrap();
    pattern
        .captures(svg)
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse::<f64>().ok())
        .unwrap_or_else(|| panic!("could not extract SVG {attr} attribute"))
}

fn svg_view_box(svg: &str) -> (f64, f64, f64, f64) {
    let pattern = Regex::new(r#"viewBox="([^"]+)""#).unwrap();
    let view_box = pattern
        .captures(svg)
        .and_then(|captures| captures.get(1))
        .unwrap_or_else(|| panic!("could not extract SVG viewBox attribute"));

    let values = view_box
        .as_str()
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .expect("viewBox should contain numeric values");

    assert_eq!(values.len(), 4, "viewBox should contain four values");
    (values[0], values[1], values[2], values[3])
}
