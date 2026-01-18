//! Journey diagram renderer
//!
//! Renders user journey diagrams showing tasks, actors, and satisfaction scores.
//! Based on the mermaid.js reference implementation.

use crate::diagrams::journey::JourneyDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

// Layout configuration (matching mermaid.js defaults)
/// Margin on the left for actor legend
const LEFT_MARGIN: f64 = 150.0;
/// Margin around the diagram
const DIAGRAM_MARGIN_X: f64 = 50.0;
const DIAGRAM_MARGIN_Y: f64 = 10.0;
/// Width of each task box
const TASK_WIDTH: f64 = 150.0;
/// Height of each task/section box
const TASK_HEIGHT: f64 = 50.0;
/// Margin between tasks
const TASK_MARGIN: f64 = 10.0;
/// Font size for text
const FONT_SIZE: f64 = 14.0;
/// Font size for title
const TITLE_FONT_SIZE: f64 = 24.0;
/// Height reserved for the title
const TITLE_HEIGHT: f64 = 50.0;
/// Section header vertical position
const SECTION_Y: f64 = 50.0;
/// Vertical position for tasks (below sections)
const TASK_Y: f64 = 100.0;
/// Face vertical base position
const FACE_BASE_Y: f64 = 300.0;
/// Face score multiplier (how much each score point moves the face)
const FACE_SCORE_MULTIPLIER: f64 = 30.0;
/// Face radius
const FACE_RADIUS: f64 = 15.0;
/// Actor colors (matching mermaid.js)
const ACTOR_COLORS: &[&str] = &[
    "#8FBC8F", "#7CFC00", "#00FFFF", "#20B2AA", "#B0E0E6", "#FFFFE0",
];
/// Section fill colors (matching mermaid.js)
const SECTION_FILLS: &[&str] = &[
    "#191970", "#8B008B", "#4B0082", "#2F4F4F", "#800000", "#8B4513", "#00008B",
];
/// Section text colors (matching mermaid.js)
const SECTION_COLORS: &[&str] = &["#fff"];

/// Render a journey diagram to SVG
pub fn render_journey(db: &JourneyDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let tasks = db.get_tasks();
    let actors = db.get_actors();
    let has_title = !db.title.is_empty();

    // Calculate dimensions
    let num_tasks = tasks.len();
    let left_margin = LEFT_MARGIN;
    let task_total_width = if num_tasks > 0 {
        (num_tasks as f64) * TASK_WIDTH + ((num_tasks - 1) as f64) * TASK_MARGIN
    } else {
        TASK_WIDTH
    };
    let width = left_margin + task_total_width + DIAGRAM_MARGIN_X * 2.0;
    let height = FACE_BASE_Y + 5.0 * FACE_SCORE_MULTIPLIER + DIAGRAM_MARGIN_Y * 2.0;

    doc.set_size(width, height);

    // Add CSS styles
    if config.embed_css {
        doc.add_style(&generate_journey_css(config));
    }

    // Add arrow marker definition
    doc.add_defs(vec![create_arrow_defs()]);

    // Build actor color map
    let actor_colors: std::collections::HashMap<String, (String, usize)> = actors
        .iter()
        .enumerate()
        .map(|(i, actor)| {
            let color = ACTOR_COLORS[i % ACTOR_COLORS.len()].to_string();
            (actor.clone(), (color, i))
        })
        .collect();

    // Render actor legend
    let legend = render_actor_legend(&actors, &actor_colors, has_title);
    doc.add_node(legend);

    // Render title if present
    if has_title {
        let title_element = render_title(&db.title, left_margin, config);
        doc.add_node(title_element);
    }

    // Render sections and tasks
    let title_offset = if has_title { TITLE_HEIGHT } else { 0.0 };
    let (sections_element, tasks_element) =
        render_sections_and_tasks(db, left_margin, title_offset, &actor_colors, config);
    doc.add_node(sections_element);
    doc.add_node(tasks_element);

    // Render activity line (arrow at the bottom)
    let line_y = TASK_Y + title_offset + TASK_HEIGHT * 2.0;
    let activity_line = render_activity_line(left_margin, line_y, task_total_width);
    doc.add_edge_path(activity_line);

    Ok(doc.to_string())
}

/// Create arrow marker definition
fn create_arrow_defs() -> SvgElement {
    SvgElement::Raw {
        content: r#"<defs>
    <marker id="arrowhead" refX="5" refY="2" markerWidth="6" markerHeight="4" orient="auto">
      <path d="M 0,0 V 4 L6,2 Z"/>
    </marker>
  </defs>"#
            .to_string(),
    }
}

/// Render the actor legend on the left side
fn render_actor_legend(
    actors: &[String],
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    has_title: bool,
) -> SvgElement {
    let mut children = Vec::new();
    let start_y = if has_title { 60.0 + TITLE_HEIGHT } else { 60.0 };

    for (i, actor) in actors.iter().enumerate() {
        let y_pos = start_y + (i as f64) * 25.0;

        // Get actor color
        let (color, pos) = actor_colors.get(actor).map(|(c, p)| (c.as_str(), *p)).unwrap_or(("#8FBC8F", 0));

        // Draw colored circle
        let circle = SvgElement::Circle {
            cx: 20.0,
            cy: y_pos,
            r: 7.0,
            attrs: Attrs::new()
                .with_class(&format!("actor-{}", pos))
                .with_fill(color)
                .with_stroke("#000"),
        };
        children.push(circle);

        // Draw actor name
        let text = SvgElement::Text {
            x: 40.0,
            y: y_pos + 5.0,
            content: actor.clone(),
            attrs: Attrs::new()
                .with_class("legend")
                .with_fill("#666")
                .with_attr("font-size", &format!("{}px", FONT_SIZE)),
        };
        children.push(text);
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("actor-legend"),
    }
}

/// Render the diagram title
fn render_title(title: &str, left_margin: f64, config: &RenderConfig) -> SvgElement {
    SvgElement::Text {
        x: left_margin,
        y: 25.0,
        content: title.to_string(),
        attrs: Attrs::new()
            .with_class("journey-title")
            .with_fill(&config.theme.primary_text_color)
            .with_attr("font-size", &format!("{}px", TITLE_FONT_SIZE))
            .with_attr("font-weight", "bold"),
    }
}

/// Render sections and tasks
fn render_sections_and_tasks(
    db: &JourneyDb,
    left_margin: f64,
    title_offset: f64,
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    _config: &RenderConfig,
) -> (SvgElement, SvgElement) {
    let tasks = db.get_tasks();
    let sections = db.get_sections();
    let mut section_elements = Vec::new();
    let mut task_elements = Vec::new();

    // If there are no tasks but there are sections, render the sections
    if tasks.is_empty() {
        for (section_idx, section_name) in sections.iter().enumerate() {
            let section_x = (section_idx as f64) * (TASK_WIDTH + TASK_MARGIN) + left_margin;
            let section_width = TASK_WIDTH;
            let fill = SECTION_FILLS[section_idx % SECTION_FILLS.len()];
            let color = SECTION_COLORS[0];
            let section_num = section_idx % SECTION_FILLS.len();

            let section = render_section(
                section_name,
                section_x,
                SECTION_Y + title_offset,
                section_width,
                TASK_HEIGHT,
                fill,
                color,
                section_num,
            );
            section_elements.push(section);
        }
    } else {
        let mut last_section = String::new();
        let mut section_number: usize = 0;

        for (i, task) in tasks.iter().enumerate() {
            // Check if we're entering a new section
            if task.section != last_section {
                // Count how many consecutive tasks share this section
                let task_count = tasks
                    .iter()
                    .skip(i)
                    .take_while(|t| t.section == task.section)
                    .count();

                // Render section header
                let section_x = (i as f64) * (TASK_WIDTH + TASK_MARGIN) + left_margin;
                let section_width = (task_count as f64) * TASK_WIDTH
                    + ((task_count.saturating_sub(1)) as f64) * TASK_MARGIN;

                let fill = SECTION_FILLS[section_number % SECTION_FILLS.len()];
                let color = SECTION_COLORS[0];
                let section_num = section_number % SECTION_FILLS.len();

                let section = render_section(
                    &task.section,
                    section_x,
                    SECTION_Y + title_offset,
                    section_width,
                    TASK_HEIGHT,
                    fill,
                    color,
                    section_num,
                );
                section_elements.push(section);

                last_section = task.section.clone();
                section_number += 1;
            }

            // Render task
            let task_x = (i as f64) * (TASK_WIDTH + TASK_MARGIN) + left_margin;
            let task_y = TASK_Y + title_offset;
            let section_num = (section_number - 1) % SECTION_FILLS.len();
            let fill = SECTION_FILLS[section_num];
            let color = SECTION_COLORS[0];

            let task_elem = render_task(
                task,
                task_x,
                task_y,
                TASK_WIDTH,
                TASK_HEIGHT,
                fill,
                color,
                section_num,
                actor_colors,
                i,
            );
            task_elements.push(task_elem);
        }
    }

    (
        SvgElement::Group {
            children: section_elements,
            attrs: Attrs::new().with_class("journey-sections"),
        },
        SvgElement::Group {
            children: task_elements,
            attrs: Attrs::new().with_class("journey-tasks"),
        },
    )
}

/// Render a section header
#[allow(clippy::too_many_arguments)]
fn render_section(
    text: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: &str,
    color: &str,
    section_num: usize,
) -> SvgElement {
    let mut children = Vec::new();

    // Section background rectangle
    let rect = SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs: Attrs::new()
            .with_class(&format!("journey-section section-type-{}", section_num))
            .with_fill(fill),
    };
    children.push(rect);

    // Section label
    let label = SvgElement::Text {
        x: x + width / 2.0,
        y: y + height / 2.0 + 5.0,
        content: text.to_string(),
        attrs: Attrs::new()
            .with_class(&format!("journey-section section-type-{}", section_num))
            .with_fill(color)
            .with_attr("text-anchor", "middle")
            .with_attr("font-size", &format!("{}px", FONT_SIZE)),
    };
    children.push(label);

    SvgElement::Group {
        children,
        attrs: Attrs::new(),
    }
}

/// Render a task with its face and actor indicators
#[allow(clippy::too_many_arguments)]
fn render_task(
    task: &crate::diagrams::journey::JourneyTask,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: &str,
    color: &str,
    section_num: usize,
    actor_colors: &std::collections::HashMap<String, (String, usize)>,
    task_index: usize,
) -> SvgElement {
    let mut children = Vec::new();

    // Task vertical line (dashed)
    let center_x = x + width / 2.0;
    let max_height = FACE_BASE_Y + 5.0 * FACE_SCORE_MULTIPLIER;
    let line = SvgElement::Line {
        x1: center_x,
        y1: y,
        x2: center_x,
        y2: max_height,
        attrs: Attrs::new()
            .with_class("task-line")
            .with_stroke("#666")
            .with_stroke_width(1.0)
            .with_stroke_dasharray("4 2"),
    };
    children.push(line);

    // Face element based on score
    let face_y = FACE_BASE_Y + ((5 - task.score) as f64) * FACE_SCORE_MULTIPLIER;
    let face = render_face(center_x, face_y, task.score);
    children.push(face);

    // Task background rectangle
    let rect = SvgElement::Rect {
        x,
        y,
        width,
        height,
        rx: Some(3.0),
        ry: Some(3.0),
        attrs: Attrs::new()
            .with_class(&format!("task task-type-{}", section_num))
            .with_fill(fill),
    };
    children.push(rect);

    // Actor circles on the task
    let mut actor_x = x + 14.0;
    for person in &task.people {
        if let Some((color, pos)) = actor_colors.get(person) {
            let circle = SvgElement::Circle {
                cx: actor_x,
                cy: y,
                r: 7.0,
                attrs: Attrs::new()
                    .with_class(&format!("actor-{}", pos))
                    .with_fill(color)
                    .with_stroke("#000")
                    .with_attr("title", person),
            };
            children.push(circle);
            actor_x += 10.0;
        }
    }

    // Task label (using foreignObject for text wrapping)
    let label = SvgElement::Raw {
        content: format!(
            r#"<foreignObject x="{}" y="{}" width="{}" height="{}"><div xmlns="http://www.w3.org/1999/xhtml" style="display:table;height:100%;width:100%;"><div class="label" style="display:table-cell;text-align:center;vertical-align:middle;color:{};">{}</div></div></foreignObject>"#,
            x, y, width, height, color, escape_xml(&task.task)
        ),
    };
    children.push(label);

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class(&format!("task-group task-{}", task_index))
            .with_id(&format!("task{}", task_index)),
    }
}

/// Render a face emoji based on score
fn render_face(cx: f64, cy: f64, score: i32) -> SvgElement {
    let mut children = Vec::new();

    // Face circle
    let face_circle = SvgElement::Circle {
        cx,
        cy,
        r: FACE_RADIUS,
        attrs: Attrs::new()
            .with_class("face")
            .with_stroke_width(2.0)
            .with_attr("overflow", "visible"),
    };
    children.push(face_circle);

    // Left eye
    let left_eye = SvgElement::Circle {
        cx: cx - FACE_RADIUS / 3.0,
        cy: cy - FACE_RADIUS / 3.0,
        r: 1.5,
        attrs: Attrs::new()
            .with_fill("#666")
            .with_stroke("#666")
            .with_stroke_width(2.0),
    };
    children.push(left_eye);

    // Right eye
    let right_eye = SvgElement::Circle {
        cx: cx + FACE_RADIUS / 3.0,
        cy: cy - FACE_RADIUS / 3.0,
        r: 1.5,
        attrs: Attrs::new()
            .with_fill("#666")
            .with_stroke("#666")
            .with_stroke_width(2.0),
    };
    children.push(right_eye);

    // Mouth based on score
    let mouth = if score > 3 {
        // Happy face - smile arc
        let inner_radius = FACE_RADIUS / 2.0;
        let outer_radius = FACE_RADIUS / 2.2;
        let path = format!(
            "M {} {} A {},{} 0 0,0 {},{}",
            cx - inner_radius,
            cy + 2.0,
            inner_radius,
            outer_radius,
            cx + inner_radius,
            cy + 2.0
        );
        SvgElement::Path {
            d: path,
            attrs: Attrs::new().with_class("mouth").with_stroke("#666").with_stroke_width(2.0).with_fill("none"),
        }
    } else if score < 3 {
        // Sad face - frown arc
        let inner_radius = FACE_RADIUS / 2.0;
        let outer_radius = FACE_RADIUS / 2.2;
        let path = format!(
            "M {} {} A {},{} 0 0,1 {},{}",
            cx - inner_radius,
            cy + 7.0,
            inner_radius,
            outer_radius,
            cx + inner_radius,
            cy + 7.0
        );
        SvgElement::Path {
            d: path,
            attrs: Attrs::new().with_class("mouth").with_stroke("#666").with_stroke_width(2.0).with_fill("none"),
        }
    } else {
        // Neutral face - straight line
        SvgElement::Line {
            x1: cx - 5.0,
            y1: cy + 7.0,
            x2: cx + 5.0,
            y2: cy + 7.0,
            attrs: Attrs::new()
                .with_class("mouth")
                .with_stroke("#666")
                .with_stroke_width(1.0),
        }
    };
    children.push(mouth);

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("face-group"),
    }
}

/// Render the activity line with arrow
fn render_activity_line(left_margin: f64, y: f64, task_width: f64) -> SvgElement {
    let x1 = left_margin;
    let x2 = left_margin + task_width - 4.0; // Subtract stroke width for arrow

    SvgElement::Line {
        x1,
        y1: y,
        x2,
        y2: y,
        attrs: Attrs::new()
            .with_class("activity-line")
            .with_stroke("black")
            .with_stroke_width(4.0)
            .with_attr("marker-end", "url(#arrowhead)"),
    }
}

/// Generate CSS for journey diagrams
fn generate_journey_css(config: &RenderConfig) -> String {
    let theme = &config.theme;

    let mut section_css = String::new();

    // Generate section type styles
    for (i, &fill) in SECTION_FILLS.iter().enumerate() {
        section_css.push_str(&format!(
            r#"
.section-type-{i} {{
  fill: {fill};
}}
.task-type-{i} {{
  fill: {fill};
}}
"#,
            i = i,
            fill = fill
        ));
    }

    // Generate actor styles
    let mut actor_css = String::new();
    for (i, &color) in ACTOR_COLORS.iter().enumerate() {
        actor_css.push_str(&format!(
            r#"
.actor-{i} {{
  fill: {color};
}}
"#,
            i = i,
            color = color
        ));
    }

    format!(
        r#"
.journey-title {{
  font-family: {font_family};
}}
.journey-section text {{
  font-family: {font_family};
}}
.task {{
  cursor: pointer;
}}
.task-line {{
  stroke: #666;
  stroke-width: 1px;
  stroke-dasharray: 4 2;
}}
.face {{
  fill: white;
  stroke: #666;
}}
.mouth {{
  stroke: #666;
}}
.legend {{
  font-family: {font_family};
}}
.label {{
  font-family: {font_family};
  font-size: {font_size}px;
}}
.activity-line {{
  fill: none;
}}
{section_css}
{actor_css}
"#,
        font_family = theme.font_family,
        font_size = FONT_SIZE,
        section_css = section_css,
        actor_css = actor_css
    )
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
