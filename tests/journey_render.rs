//! Journey diagram rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/integration/rendering/journey.spec.js

use roxmltree::Document;
use selkie::{parse, render};

fn render_journey_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse journey diagram");
    render(&diagram).expect("Failed to render journey diagram")
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

/// Helper to check basic SVG structure
fn assert_valid_svg(svg: &str) {
    assert!(svg.contains("<svg"), "SVG should have opening tag");
    assert!(svg.contains("</svg>"), "SVG should have closing tag");
    assert!(
        svg.contains("xmlns=\"http://www.w3.org/2000/svg\""),
        "SVG should have namespace"
    );
}

// ============================================================================
// Basic Rendering Tests (from Cypress journey.spec.js)
// ============================================================================

#[test]
fn journey_simple_test() {
    // From Cypress: Simple test
    let input = r#"journey
title Adding journey diagram functionality to mermaid
section Order from website"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    // Should contain the title
    assert!(
        svg_contains_text(&svg, "Adding journey diagram functionality to mermaid"),
        "Should contain title text"
    );

    // Should contain the section
    assert!(
        svg_contains_text(&svg, "Order from website"),
        "Should contain section text"
    );
}

#[test]
fn journey_user_journey_chart() {
    // From Cypress: should render a user journey chart
    let input = r#"journey
    title My working day
    section Go to work
      Make tea: 5: Me
      Go upstairs: 3: Me
      Do work: 1: Me, Cat
    section Go home
      Go downstairs: 5: Me
      Sit down: 3: Me"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    // Should contain title
    assert!(
        svg_contains_text(&svg, "My working day"),
        "Should contain title"
    );

    // Should contain sections
    assert!(
        svg_contains_text(&svg, "Go to work"),
        "Should contain 'Go to work' section"
    );
    assert!(
        svg_contains_text(&svg, "Go home"),
        "Should contain 'Go home' section"
    );

    // Should contain task names
    assert!(
        svg_contains_text(&svg, "Make tea"),
        "Should contain 'Make tea' task"
    );
    assert!(
        svg_contains_text(&svg, "Do work"),
        "Should contain 'Do work' task"
    );
}

#[test]
fn journey_e_commerce() {
    // From Cypress: E-Commerce journey
    let input = r#"journey
title E-Commerce
section Order from website
  Add to cart: 5: Me
section Checkout from website
  Add payment details: 5: Me"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    assert!(
        svg_contains_text(&svg, "E-Commerce"),
        "Should contain title"
    );
    assert!(
        svg_contains_text(&svg, "Add to cart"),
        "Should contain 'Add to cart' task"
    );
    assert!(
        svg_contains_text(&svg, "Add payment details"),
        "Should contain 'Add payment details' task"
    );
}

#[test]
fn journey_multiple_actors() {
    // From Cypress: multiple actors (Web hook life cycle)
    let input = r#"journey
title Web hook life cycle
section Darkoob
  Make preBuilt:5: Darkoob user
  register slug : 5: Darkoob admin
  Map slug to a Prebuilt Job:5: Darkoob user
section External Service
  set Darkoob slug as hook for an Event : 5 : admin
  listen to the events : 5 :  External Service
  call darkoob endpoint : 5 : External Service"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    assert!(
        svg_contains_text(&svg, "Web hook life cycle"),
        "Should contain title"
    );

    // Should have journey-section classes for different sections
    let doc = parse_svg(&svg);
    assert!(
        has_class(&doc, "journey-section"),
        "Should have journey-section class"
    );
}

#[test]
fn journey_tasks_without_actors() {
    // From Cypress: tasks without explicit actors
    let input = r#"journey
    title User Journey Example
    section Onboarding
        Sign Up: 5:
        Browse Features: 3:
        Use Core Functionality: 4:"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    assert!(
        svg_contains_text(&svg, "User Journey Example"),
        "Should contain title"
    );
    assert!(
        svg_contains_text(&svg, "Sign Up"),
        "Should contain 'Sign Up' task"
    );
}

// ============================================================================
// Structure and Layout Tests
// ============================================================================

#[test]
fn journey_has_sections() {
    let input = r#"journey
title Test Journey
section First Section
  Task 1: 5: Actor1
section Second Section
  Task 2: 3: Actor1"#;

    let svg = render_journey_svg(input);
    let doc = parse_svg(&svg);

    // Should have section elements with journey-section class
    assert!(
        has_class(&doc, "journey-section"),
        "Should have journey-section class"
    );

    // Count sections (should be 2)
    let section_count = count_elements_with_class(&doc, "journey-section");
    assert!(
        section_count >= 2,
        "Should have at least 2 sections, found {}",
        section_count
    );
}

#[test]
fn journey_has_tasks() {
    let input = r#"journey
title Test Journey
section Test Section
  Task 1: 5: Actor1
  Task 2: 4: Actor1
  Task 3: 3: Actor1"#;

    let svg = render_journey_svg(input);
    let doc = parse_svg(&svg);

    // Should have task elements
    assert!(has_class(&doc, "task"), "Should have task class");

    // Count tasks (should be 3)
    let task_count = count_elements_with_class(&doc, "task");
    assert!(
        task_count >= 3,
        "Should have at least 3 tasks, found {}",
        task_count
    );
}

#[test]
fn journey_has_actor_legend() {
    let input = r#"journey
title Test Journey
section Test Section
  Task 1: 5: Alice
  Task 2: 4: Bob
  Task 3: 3: Alice, Bob"#;

    let svg = render_journey_svg(input);

    // Should contain actor names in legend
    assert!(svg_contains_text(&svg, "Alice"), "Should contain 'Alice'");
    assert!(svg_contains_text(&svg, "Bob"), "Should contain 'Bob'");
}

// ============================================================================
// Score/Face Tests
// ============================================================================

#[test]
fn journey_score_faces() {
    // Journey diagrams show faces based on scores:
    // - Score > 3: happy face (smile)
    // - Score < 3: sad face
    // - Score == 3: neutral face (straight line)
    let input = r#"journey
title Score Test
section Test Section
  Happy Task: 5: Actor
  Sad Task: 1: Actor
  Neutral Task: 3: Actor"#;

    let svg = render_journey_svg(input);
    let doc = parse_svg(&svg);

    // Should have face elements
    assert!(has_class(&doc, "face"), "Should have face class for scores");
}

// ============================================================================
// Theme Tests
// ============================================================================

#[test]
fn journey_section_colors() {
    // Sections should have alternating colors (section-type-0, section-type-1, etc.)
    let input = r#"journey
title Color Test
section Section 1
  Task 1: 5: Actor
section Section 2
  Task 2: 4: Actor
section Section 3
  Task 3: 3: Actor"#;

    let svg = render_journey_svg(input);
    let doc = parse_svg(&svg);

    // Should have section-type classes for coloring
    assert!(
        has_class(&doc, "section-type-0"),
        "Should have section-type-0 class"
    );
    assert!(
        has_class(&doc, "section-type-1"),
        "Should have section-type-1 class"
    );
}

#[test]
fn journey_actor_colors() {
    // Actors should have different colors
    let input = r#"journey
title Actor Color Test
section Test
  Task 1: 5: Actor1
  Task 2: 4: Actor2
  Task 3: 3: Actor3"#;

    let svg = render_journey_svg(input);
    let doc = parse_svg(&svg);

    // Should have actor classes for coloring
    assert!(has_class(&doc, "actor-0"), "Should have actor-0 class");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn journey_empty_section() {
    // A section with no tasks
    let input = r#"journey
title Empty Section Test
section Empty Section
section Section with Task
  Task 1: 5: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    assert!(
        svg_contains_text(&svg, "Empty Section"),
        "Should contain empty section name"
    );
}

#[test]
fn journey_no_title() {
    // Journey without explicit title
    let input = r#"journey
section Test Section
  Task 1: 5: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);
}

#[test]
fn journey_special_characters_in_task() {
    // Tasks with special characters
    let input = r#"journey
title Special Characters
section Test
  Task with special chars!: 5: Actor
  Another & task: 4: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    // Special characters should be escaped in SVG
    assert!(
        svg_contains_text(&svg, "Special Characters"),
        "Should contain title"
    );
}

#[test]
fn journey_long_task_names() {
    // Tasks with long names
    let input = r#"journey
title Long Task Names
section Test
  This is a very long task name that should be displayed properly in the diagram: 5: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);
}

#[test]
fn journey_many_actors() {
    // Journey with many actors
    let input = r#"journey
title Many Actors
section Test
  Task 1: 5: Actor1, Actor2, Actor3, Actor4, Actor5"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    // All actors should be present
    assert!(svg_contains_text(&svg, "Actor1"), "Should contain Actor1");
    assert!(svg_contains_text(&svg, "Actor5"), "Should contain Actor5");
}

// ============================================================================
// Accessibility Tests
// ============================================================================

#[test]
fn journey_with_accessibility() {
    let input = r#"journey
accTitle: Journey Accessibility Title
accDescr: This is a description of the journey
title My Journey
section Test
  Task: 5: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    // The diagram should render successfully with accessibility attributes
    assert!(
        svg_contains_text(&svg, "My Journey"),
        "Should contain title"
    );
}

// ============================================================================
// Complex Example Tests
// ============================================================================

#[test]
fn journey_complex_example() {
    // A more complex journey diagram
    let input = r#"journey
title Online Shopping Experience
section Discovery
  Search for product: 4: Customer
  Browse recommendations: 3: Customer
  Read reviews: 5: Customer
section Purchase
  Add to cart: 5: Customer
  Enter shipping details: 3: Customer
  Make payment: 4: Customer, Payment System
section Fulfillment
  Order confirmed: 5: Customer, System
  Package shipped: 4: Warehouse
  Delivery: 5: Customer, Courier"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    let doc = parse_svg(&svg);

    // Should have multiple sections
    let section_count = count_elements_with_class(&doc, "journey-section");
    assert!(
        section_count >= 3,
        "Should have at least 3 sections, found {}",
        section_count
    );

    // Should have multiple tasks
    let task_count = count_elements_with_class(&doc, "task");
    assert!(
        task_count >= 9,
        "Should have at least 9 tasks, found {}",
        task_count
    );
}

#[test]
fn journey_all_scores() {
    // Test all score values (1-5)
    let input = r#"journey
title Score Range Test
section Test
  Score 1: 1: Actor
  Score 2: 2: Actor
  Score 3: 3: Actor
  Score 4: 4: Actor
  Score 5: 5: Actor"#;

    let svg = render_journey_svg(input);
    assert_valid_svg(&svg);

    let doc = parse_svg(&svg);

    // Should have face elements for each task
    let face_count = count_elements_with_class(&doc, "face");
    assert!(
        face_count >= 5,
        "Should have at least 5 faces, found {}",
        face_count
    );
}
