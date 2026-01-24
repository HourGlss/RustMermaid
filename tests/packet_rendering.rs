//! Packet diagram rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/integration/rendering/packet.spec.ts

use roxmltree::Document;
use selkie::{parse, render};

fn render_packet_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse packet diagram");
    render(&diagram).expect("Failed to render packet diagram")
}

fn parse_svg(svg: &str) -> Document<'_> {
    Document::parse(svg).expect("Failed to parse SVG")
}

fn has_class(doc: &Document<'_>, class_name: &str) -> bool {
    doc.descendants().any(|node| {
        node.attribute("class")
            .map(|class| class.split_whitespace().any(|c| c == class_name))
            .unwrap_or(false)
    })
}

fn count_elements_with_class(doc: &Document<'_>, class_name: &str) -> usize {
    doc.descendants()
        .filter(|node| {
            node.attribute("class")
                .map(|class| class.split_whitespace().any(|c| c == class_name))
                .unwrap_or(false)
        })
        .count()
}

fn svg_contains_text(svg: &str, text: &str) -> bool {
    svg.contains(text)
}

// ============================================================================
// Basic Tests (from packet.spec.ts)
// ============================================================================

#[test]
fn packet_should_render_simple_packet_beta_diagram() {
    let input = r#"packet-beta
  title Hello world
  0-10: "hello"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have packetBlock class
    assert!(
        has_class(&doc, "packetBlock"),
        "Should have packetBlock class"
    );

    // Should contain the label text
    assert!(
        svg_contains_text(&svg, "hello"),
        "Should contain 'hello' label"
    );

    // Should contain the title
    assert!(
        svg_contains_text(&svg, "Hello world"),
        "Should contain title 'Hello world'"
    );
}

#[test]
fn packet_should_render_simple_packet_diagram() {
    let input = r#"packet
  title Hello world
  0-10: "hello"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have packetBlock class
    assert!(
        has_class(&doc, "packetBlock"),
        "Should have packetBlock class"
    );

    // Should contain the label text
    assert!(
        svg_contains_text(&svg, "hello"),
        "Should contain 'hello' label"
    );

    // Should contain the title
    assert!(
        svg_contains_text(&svg, "Hello world"),
        "Should contain title 'Hello world'"
    );
}

#[test]
fn packet_should_render_diagram_without_ranges() {
    let input = r#"packet
  0: "h"
  1: "i"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have packetBlock class
    assert!(
        has_class(&doc, "packetBlock"),
        "Should have packetBlock class"
    );

    // Should have 2 blocks (2 rects)
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(block_count, 2, "Should have 2 packet blocks");

    // Should contain the labels
    assert!(svg_contains_text(&svg, ">h<"), "Should contain 'h' label");
    assert!(svg_contains_text(&svg, ">i<"), "Should contain 'i' label");
}

#[test]
fn packet_should_render_complex_packet_diagram() {
    let input = r#"packet
    0-15: "Source Port"
    16-31: "Destination Port"
    32-63: "Sequence Number"
    64-95: "Acknowledgment Number"
    96-99: "Data Offset"
    100-105: "Reserved"
    106: "URG"
    107: "ACK"
    108: "PSH"
    109: "RST"
    110: "SYN"
    111: "FIN"
    112-127: "Window"
    128-143: "Checksum"
    144-159: "Urgent Pointer"
    160-191: "(Options and Padding)"
    192-223: "data"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have packetBlock class
    assert!(
        has_class(&doc, "packetBlock"),
        "Should have packetBlock class"
    );

    // Should contain key TCP header fields
    assert!(
        svg_contains_text(&svg, "Source Port"),
        "Should contain 'Source Port'"
    );
    assert!(
        svg_contains_text(&svg, "Destination Port"),
        "Should contain 'Destination Port'"
    );
    assert!(
        svg_contains_text(&svg, "Sequence Number"),
        "Should contain 'Sequence Number'"
    );
    assert!(
        svg_contains_text(&svg, "Acknowledgment Number"),
        "Should contain 'Acknowledgment Number'"
    );
    assert!(svg_contains_text(&svg, "URG"), "Should contain 'URG'");
    assert!(svg_contains_text(&svg, "ACK"), "Should contain 'ACK'");
    assert!(svg_contains_text(&svg, "SYN"), "Should contain 'SYN'");
    assert!(svg_contains_text(&svg, "FIN"), "Should contain 'FIN'");
    assert!(svg_contains_text(&svg, "Window"), "Should contain 'Window'");
    assert!(
        svg_contains_text(&svg, "Checksum"),
        "Should contain 'Checksum'"
    );

    // Should have bit numbers displayed (showBits is true by default)
    assert!(
        has_class(&doc, "packetByte"),
        "Should have packetByte class for bit numbers"
    );
}

#[test]
fn packet_should_render_with_multiple_rows() {
    // This tests that blocks spanning multiple rows are properly split
    let input = r#"packet
    0-31: "First Row"
    32-63: "Second Row"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have 2 rows of blocks
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(block_count, 2, "Should have 2 packet blocks");

    assert!(
        svg_contains_text(&svg, "First Row"),
        "Should contain 'First Row'"
    );
    assert!(
        svg_contains_text(&svg, "Second Row"),
        "Should contain 'Second Row'"
    );
}

#[test]
fn packet_should_render_block_split_across_rows() {
    // Test a block that spans across row boundary
    let input = r#"packet
    0-16: "test"
    17-63: "multiple"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // The "multiple" block gets split into:
    // - Row 0: bits 17-31 (part of first 32 bits)
    // - Row 1: bits 32-63 (full second row)
    // So we should have 3 blocks total: test, multiple (part 1), multiple (part 2)
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(
        block_count, 3,
        "Should have 3 packet blocks (1 + 2 split parts)"
    );

    assert!(svg_contains_text(&svg, "test"), "Should contain 'test'");
    assert!(
        svg_contains_text(&svg, "multiple"),
        "Should contain 'multiple'"
    );
}

#[test]
fn packet_should_display_bit_numbers() {
    let input = r#"packet
    0-7: "byte"
    8-15: "byte2"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have packetByte class for bit numbers
    assert!(
        has_class(&doc, "packetByte"),
        "Should have packetByte class"
    );

    // Should show start bit numbers
    assert!(
        svg_contains_text(&svg, ">0<"),
        "Should contain bit number 0"
    );
    assert!(
        svg_contains_text(&svg, ">7<"),
        "Should contain bit number 7"
    );
    assert!(
        svg_contains_text(&svg, ">8<"),
        "Should contain bit number 8"
    );
    assert!(
        svg_contains_text(&svg, ">15<"),
        "Should contain bit number 15"
    );
}

#[test]
fn packet_should_handle_single_bit_blocks() {
    let input = r#"packet
    0-10: "test"
    11: "single"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have 2 blocks
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(block_count, 2, "Should have 2 packet blocks");

    assert!(svg_contains_text(&svg, "test"), "Should contain 'test'");
    assert!(svg_contains_text(&svg, "single"), "Should contain 'single'");

    // Single bit block should center the bit number
    assert!(
        svg_contains_text(&svg, ">11<"),
        "Should contain bit number 11"
    );
}

#[test]
fn packet_should_handle_bit_count_notation() {
    let input = r#"packet
    +8: "byte"
    +16: "word"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should have 2 blocks
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(block_count, 2, "Should have 2 packet blocks");

    assert!(svg_contains_text(&svg, "byte"), "Should contain 'byte'");
    assert!(svg_contains_text(&svg, "word"), "Should contain 'word'");

    // Check bit numbers: byte is 0-7, word is 8-23
    assert!(
        svg_contains_text(&svg, ">0<"),
        "Should contain bit number 0"
    );
    assert!(
        svg_contains_text(&svg, ">7<"),
        "Should contain bit number 7"
    );
    assert!(
        svg_contains_text(&svg, ">8<"),
        "Should contain bit number 8"
    );
    assert!(
        svg_contains_text(&svg, ">23<"),
        "Should contain bit number 23"
    );
}

#[test]
fn packet_empty_diagram_renders() {
    let input = "packet";
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    // Should produce valid SVG even with no blocks
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Should have no blocks
    let block_count = count_elements_with_class(&doc, "packetBlock");
    assert_eq!(block_count, 0, "Should have 0 packet blocks");
}

#[test]
fn packet_title_renders_at_bottom() {
    let input = r#"packet
    title My Packet
    0-7: "data"
"#;
    let svg = render_packet_svg(input);

    // Title should be present
    assert!(
        svg_contains_text(&svg, "My Packet"),
        "Should contain title 'My Packet'"
    );

    // Title should have packetTitle class
    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "packetTitle"),
        "Should have packetTitle class"
    );
}

// ============================================================================
// Mermaid Visual Parity Tests
// ============================================================================

fn get_svg_dimensions(doc: &Document<'_>) -> (f64, f64) {
    let svg_node = doc
        .descendants()
        .find(|n| n.tag_name().name() == "svg")
        .expect("SVG element not found");
    let width: f64 = svg_node
        .attribute("width")
        .expect("width attribute missing")
        .parse()
        .expect("invalid width");
    let height: f64 = svg_node
        .attribute("height")
        .expect("height attribute missing")
        .parse()
        .expect("invalid height");
    (width, height)
}

fn get_first_rect_y(doc: &Document<'_>) -> f64 {
    let rect = doc
        .descendants()
        .find(|n| n.tag_name().name() == "rect" && n.has_attribute("y"))
        .expect("No rect found");
    rect.attribute("y")
        .expect("y attribute missing")
        .parse()
        .expect("invalid y")
}

fn get_first_rect_width(doc: &Document<'_>) -> f64 {
    let rect = doc
        .descendants()
        .find(|n| n.tag_name().name() == "rect" && n.has_attribute("width"))
        .expect("No rect found");
    rect.attribute("width")
        .expect("width attribute missing")
        .parse()
        .expect("invalid width")
}

#[test]
fn packet_dimensions_match_mermaid_with_title() {
    // Test case: TCP packet with title (from eval packet_complex)
    // Mermaid calculates:
    //   rowHeight: 32, paddingY: 5 (base) + 10 (showBits) = 15, bitWidth: 32, bitsPerRow: 32
    //   totalRowHeight = 32 + 15 = 47
    //   For 9 rows (bits 0-255 = 8 rows) + 1 title row: height = 47 * 9 = 423
    //   Width = 32 * 32 + 2 = 1026
    let input = r#"packet
    title TCP Packet Structure
    0-15: "Source Port"
    16-31: "Destination Port"
    32-63: "Sequence Number"
    64-95: "Acknowledgment Number"
    96-99: "Data Offset"
    100-105: "Reserved"
    106: "URG"
    107: "ACK"
    108: "PSH"
    109: "RST"
    110: "SYN"
    111: "FIN"
    112-127: "Window"
    128-143: "Checksum"
    144-159: "Urgent Pointer"
    160-191: "(Options and Padding)"
    192-255: "Data"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    let (width, height) = get_svg_dimensions(&doc);

    // Width should be bitWidth * bitsPerRow + 2 = 32 * 32 + 2 = 1026
    assert_eq!(width, 1026.0, "SVG width should match mermaid (1026)");

    // Height should be totalRowHeight * (rows + 1) = 47 * 9 = 423
    // (9 rows for 256 bits / 32 bits_per_row, + title row space, but actually
    // the formula is: totalRowHeight * (words.length + 1) - (title ? 0 : rowHeight)
    // With title: 47 * 9 = 423
    assert_eq!(height, 423.0, "SVG height should match mermaid (423)");
}

#[test]
fn packet_first_row_y_position_matches_mermaid() {
    // First row should start at y = paddingY (15 when showBits is true)
    let input = r#"packet
    0-15: "Test"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    let first_y = get_first_rect_y(&doc);

    // With showBits=true, paddingY = 5 + 10 = 15
    assert_eq!(
        first_y, 15.0,
        "First row Y should be paddingY (15 with showBits)"
    );
}

#[test]
fn packet_block_width_matches_mermaid() {
    // A 16-bit block should have width = 16 * bitWidth - paddingX = 16 * 32 - 5 = 507
    let input = r#"packet
    0-15: "Test"
"#;
    let svg = render_packet_svg(input);
    let doc = parse_svg(&svg);

    let first_width = get_first_rect_width(&doc);

    // Width = bits * bitWidth - paddingX = 16 * 32 - 5 = 507
    assert_eq!(
        first_width, 507.0,
        "16-bit block width should be 507 (16*32-5)"
    );
}
