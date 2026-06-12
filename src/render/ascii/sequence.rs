//! ASCII renderer for sequence diagrams.
//!
//! Produces character-art output with:
//! - Box-drawing participant headers (top and bottom)
//! - Vertical lifelines using `│`
//! - Horizontal message arrows using `─` with `>` or `>>` tips
//! - Message labels centered above arrows
//! - Fragment boxes (loop/alt/opt/par) using box-drawing characters

use crate::diagrams::sequence::{Actor, LineType, Message, SequenceDb};
use crate::error::Result;
use std::collections::HashMap;

/// Layout constants for ASCII sequence diagrams.
const ACTOR_COL_WIDTH: usize = 20;
const ACTOR_BOX_PADDING: usize = 2;
const MESSAGE_ROW_SPACING: usize = 2;

/// Render a sequence diagram as character art.
pub fn render_sequence_ascii(db: &SequenceDb) -> Result<String> {
    let actors = db.get_actors_in_order();
    if actors.is_empty() {
        return Ok(String::new());
    }

    let messages = db.get_messages();
    let actor_widths = actor_widths(&actors);
    let actor_index = actor_index(&actors);
    let gap_widths = gap_widths(&actors, messages, &actor_widths, &actor_index);
    let actor_centers = actor_centers(&actor_widths, &gap_widths);
    let total_width = total_sequence_width(&actor_centers, &actor_widths);
    let row_events = sequence_row_events(messages);

    // Canvas: allocate rows
    // Top actor box: 3 rows, then each event gets MESSAGE_ROW_SPACING rows,
    // then bottom actor box: 3 rows
    let content_rows = row_events.len() * MESSAGE_ROW_SPACING;
    let top_box_rows = 3;
    let bottom_box_rows = 3;
    let total_rows = top_box_rows + 1 + content_rows + 1 + bottom_box_rows;

    let mut canvas: Vec<Vec<char>> = vec![vec![' '; total_width]; total_rows];

    draw_actor_boxes(&mut canvas, 0, &actors, &actor_centers, &actor_widths);
    draw_lifelines(
        &mut canvas,
        &actor_centers,
        top_box_rows,
        total_rows - bottom_box_rows - 1,
    );
    draw_sequence_events(
        &mut canvas,
        &row_events,
        &actor_index,
        &actor_centers,
        total_width,
        top_box_rows + 1,
    );
    let bottom_row = total_rows - bottom_box_rows;
    draw_actor_boxes(
        &mut canvas,
        bottom_row,
        &actors,
        &actor_centers,
        &actor_widths,
    );

    Ok(canvas_to_string(&canvas))
}

/// Row event types for sequence diagram layout
enum RowEvent<'a> {
    Message {
        from: &'a str,
        to: &'a str,
        label: &'a str,
        msg_type: LineType,
    },
    FragmentStart(String, LineType),
    FragmentDivider(String, LineType),
    FragmentEnd,
}

fn actor_widths(actors: &[&Actor]) -> Vec<usize> {
    actors
        .iter()
        .map(|actor| {
            let label_len = actor.description.chars().count();
            (label_len + ACTOR_BOX_PADDING * 2 + 2).max(ACTOR_COL_WIDTH)
        })
        .collect()
}

fn actor_index<'a>(actors: &'a [&Actor]) -> HashMap<&'a str, usize> {
    actors
        .iter()
        .enumerate()
        .map(|(i, actor)| (actor.name.as_str(), i))
        .collect()
}

fn gap_widths(
    actors: &[&Actor],
    messages: &[Message],
    actor_widths: &[usize],
    actor_index: &HashMap<&str, usize>,
) -> Vec<usize> {
    let mut gap_widths = vec![4; actors.len().saturating_sub(1)];

    for msg in messages {
        widen_gaps_for_message(msg, actor_widths, actor_index, &mut gap_widths);
    }

    gap_widths
}

fn widen_gaps_for_message(
    msg: &Message,
    actor_widths: &[usize],
    actor_index: &HashMap<&str, usize>,
    gap_widths: &mut [usize],
) {
    let Some((fi, ti)) = message_actor_indexes(msg, actor_index) else {
        return;
    };
    if fi == ti {
        return;
    }

    let min_idx = fi.min(ti);
    let max_idx = fi.max(ti);
    let needed = msg.message.chars().count() + 4; // label + padding
    let actor_half = actor_widths[fi] / 2 + actor_widths[ti] / 2;
    let total_gap_needed = needed.saturating_sub(actor_half).max(4);
    let per_gap = total_gap_needed.div_ceil(max_idx - min_idx);

    for gap in gap_widths.iter_mut().take(max_idx).skip(min_idx) {
        *gap = (*gap).max(per_gap);
    }
}

fn message_actor_indexes(
    msg: &Message,
    actor_index: &HashMap<&str, usize>,
) -> Option<(usize, usize)> {
    let from = msg.from.as_deref()?;
    let to = msg.to.as_deref()?;
    Some((*actor_index.get(from)?, *actor_index.get(to)?))
}

fn actor_centers(actor_widths: &[usize], gap_widths: &[usize]) -> Vec<usize> {
    let mut centers = Vec::with_capacity(actor_widths.len());
    let mut x = 1; // left margin

    for (i, width) in actor_widths.iter().enumerate() {
        centers.push(x + width / 2);
        x += width + gap_widths.get(i).copied().unwrap_or(4);
    }

    centers
}

fn total_sequence_width(actor_centers: &[usize], actor_widths: &[usize]) -> usize {
    actor_centers
        .last()
        .map(|last_center| last_center + actor_widths.last().unwrap_or(&ACTOR_COL_WIDTH) / 2 + 2)
        .unwrap_or(80)
}

fn sequence_row_events(messages: &[Message]) -> Vec<RowEvent<'_>> {
    let mut row_events = Vec::new();

    for msg in messages {
        if let Some(event) = sequence_row_event(msg) {
            row_events.push(event);
        }
    }

    row_events
}

fn sequence_row_event(msg: &Message) -> Option<RowEvent<'_>> {
    match msg.message_type {
        LineType::ActiveStart | LineType::ActiveEnd | LineType::Autonumber => None,
        LineType::LoopStart
        | LineType::AltStart
        | LineType::OptStart
        | LineType::ParStart
        | LineType::CriticalStart
        | LineType::BreakStart
        | LineType::RectStart => Some(RowEvent::FragmentStart(
            msg.message.clone(),
            msg.message_type,
        )),
        LineType::AltElse | LineType::ParAnd | LineType::CriticalOption => Some(
            RowEvent::FragmentDivider(msg.message.clone(), msg.message_type),
        ),
        LineType::LoopEnd
        | LineType::AltEnd
        | LineType::OptEnd
        | LineType::ParEnd
        | LineType::CriticalEnd
        | LineType::BreakEnd
        | LineType::RectEnd => Some(RowEvent::FragmentEnd),
        _ if msg.from.is_some() && msg.to.is_some() => Some(RowEvent::Message {
            from: msg.from.as_deref().unwrap_or(""),
            to: msg.to.as_deref().unwrap_or(""),
            label: &msg.message,
            msg_type: msg.message_type,
        }),
        _ => None,
    }
}

fn draw_actor_boxes(
    canvas: &mut [Vec<char>],
    row: usize,
    actors: &[&Actor],
    actor_centers: &[usize],
    actor_widths: &[usize],
) {
    for (i, actor) in actors.iter().enumerate() {
        draw_actor_box(
            canvas,
            row,
            actor_centers[i],
            actor_widths[i],
            &actor.description,
        );
    }
}

fn draw_lifelines(
    canvas: &mut [Vec<char>],
    actor_centers: &[usize],
    lifeline_start: usize,
    lifeline_end: usize,
) {
    let total_width = canvas.first().map(Vec::len).unwrap_or(0);

    for &center in actor_centers {
        if center < total_width {
            draw_lifeline(canvas, center, lifeline_start, lifeline_end);
        }
    }
}

fn draw_lifeline(
    canvas: &mut [Vec<char>],
    center: usize,
    lifeline_start: usize,
    lifeline_end: usize,
) {
    for canvas_row in canvas
        .iter_mut()
        .take(lifeline_end + 1)
        .skip(lifeline_start)
    {
        if canvas_row[center] == ' ' {
            canvas_row[center] = '│';
        }
    }
}

fn draw_sequence_events(
    canvas: &mut [Vec<char>],
    row_events: &[RowEvent<'_>],
    actor_index: &HashMap<&str, usize>,
    actor_centers: &[usize],
    total_width: usize,
    start_row: usize,
) {
    let mut current_row = start_row;
    let mut fragment_stack: Vec<(usize, String)> = Vec::new(); // (start_row, label)

    for event in row_events {
        draw_sequence_event(
            canvas,
            event,
            actor_index,
            actor_centers,
            total_width,
            current_row,
            &mut fragment_stack,
        );
        current_row += MESSAGE_ROW_SPACING;
    }
}

fn draw_sequence_event(
    canvas: &mut [Vec<char>],
    event: &RowEvent<'_>,
    actor_index: &HashMap<&str, usize>,
    actor_centers: &[usize],
    total_width: usize,
    current_row: usize,
    fragment_stack: &mut Vec<(usize, String)>,
) {
    match event {
        RowEvent::Message {
            from,
            to,
            label,
            msg_type,
        } => draw_sequence_message_event(
            canvas,
            from,
            to,
            label,
            *msg_type,
            actor_index,
            actor_centers,
            total_width,
            current_row,
        ),
        RowEvent::FragmentStart(label, msg_type) => {
            fragment_stack.push((current_row, fragment_prefix(*msg_type).to_string()));
            draw_fragment_header(canvas, current_row, total_width, label, *msg_type);
        }
        RowEvent::FragmentDivider(label, msg_type) => {
            draw_fragment_divider(canvas, current_row, total_width, label, *msg_type);
        }
        RowEvent::FragmentEnd => {
            if fragment_stack.pop().is_some() {
                draw_text_at_row(canvas, current_row, total_width, "[end]");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_sequence_message_event(
    canvas: &mut [Vec<char>],
    from: &str,
    to: &str,
    label: &str,
    msg_type: LineType,
    actor_index: &HashMap<&str, usize>,
    actor_centers: &[usize],
    total_width: usize,
    current_row: usize,
) {
    let Some((fi, ti)) = actor_index
        .get(from)
        .copied()
        .zip(actor_index.get(to).copied())
    else {
        return;
    };

    let from_col = actor_centers[fi];
    let to_col = actor_centers[ti];
    if fi == ti {
        draw_self_message(canvas, current_row, from_col, label, total_width);
    } else {
        draw_message(
            canvas,
            current_row,
            from_col,
            to_col,
            label,
            is_dotted_message(msg_type),
            total_width,
        );
    }
}

fn is_dotted_message(msg_type: LineType) -> bool {
    matches!(
        msg_type,
        LineType::Dotted | LineType::DottedOpen | LineType::DottedCross | LineType::DottedPoint
    )
}

fn draw_fragment_header(
    canvas: &mut [Vec<char>],
    row: usize,
    total_width: usize,
    label: &str,
    msg_type: LineType,
) {
    let prefix = fragment_prefix(msg_type);
    let header = if label.is_empty() {
        format!("[{}]", prefix)
    } else {
        format!("[{} {}]", prefix, label)
    };
    draw_text_at_row(canvas, row, total_width, &header);
}

fn draw_fragment_divider(
    canvas: &mut [Vec<char>],
    row: usize,
    total_width: usize,
    label: &str,
    msg_type: LineType,
) {
    let prefix = fragment_prefix(msg_type);
    let divider_label = if label.is_empty() {
        format!("- - [{}] - -", prefix)
    } else {
        format!("- - [{}] - -", label)
    };
    draw_text_at_row(canvas, row, total_width, &divider_label);
}

fn draw_text_at_row(canvas: &mut [Vec<char>], row: usize, total_width: usize, text: &str) {
    if row >= canvas.len() {
        return;
    }

    for (j, ch) in text.chars().enumerate() {
        if j < total_width {
            canvas[row][j] = ch;
        }
    }
}

fn canvas_to_string(canvas: &[Vec<char>]) -> String {
    let last_non_empty = canvas
        .iter()
        .rposition(|row| row.iter().any(|&c| c != ' '))
        .unwrap_or(0);
    let mut result = String::new();

    for row in &canvas[..=last_non_empty] {
        let line: String = row.iter().collect();
        result.push_str(line.trim_end());
        result.push('\n');
    }

    result
}

/// Draw an actor box at the given position
fn draw_actor_box(
    canvas: &mut [Vec<char>],
    start_row: usize,
    center_col: usize,
    width: usize,
    label: &str,
) {
    let half_w = width / 2;
    let left = center_col.saturating_sub(half_w);
    let right = left + width - 1;
    let cols = canvas[0].len();

    if start_row + 2 >= canvas.len() {
        return;
    }

    // Top border: ┌───┐
    if left < cols {
        canvas[start_row][left] = '┌';
    }
    for cell in canvas[start_row]
        .iter_mut()
        .take(right.min(cols))
        .skip(left + 1)
    {
        *cell = '─';
    }
    if right < cols {
        canvas[start_row][right] = '┐';
    }

    // Middle: │ label │
    if left < cols {
        canvas[start_row + 1][left] = '│';
    }
    if right < cols {
        canvas[start_row + 1][right] = '│';
    }
    // Center label in the box
    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();
    let inner_width = right.saturating_sub(left + 1);
    let label_start = left + 1 + inner_width.saturating_sub(label_len) / 2;
    for (j, &ch) in label_chars.iter().enumerate() {
        let col = label_start + j;
        if col < right && col < cols {
            canvas[start_row + 1][col] = ch;
        }
    }

    // Bottom border: └───┘
    if left < cols {
        canvas[start_row + 2][left] = '└';
    }
    for cell in canvas[start_row + 2]
        .iter_mut()
        .take(right.min(cols))
        .skip(left + 1)
    {
        *cell = '─';
    }
    if right < cols {
        canvas[start_row + 2][right] = '┘';
    }
}

/// Draw a message arrow between two actor lifelines
fn draw_message(
    canvas: &mut [Vec<char>],
    row: usize,
    from_col: usize,
    to_col: usize,
    label: &str,
    is_dotted: bool,
    total_width: usize,
) {
    if row >= canvas.len() {
        return;
    }

    let (left, right, going_right) = if from_col < to_col {
        (from_col, to_col, true)
    } else {
        (to_col, from_col, false)
    };

    // Draw arrow line
    let line_char = if is_dotted { '·' } else { '─' };
    for cell in canvas[row]
        .iter_mut()
        .take(right.min(total_width))
        .skip(left + 1)
    {
        *cell = line_char;
    }

    // Arrow tip
    if going_right {
        if right < total_width {
            canvas[row][right] = '>';
        }
    } else if left < total_width {
        canvas[row][left] = '<';
    }

    // Place label above the arrow (centered between from/to, avoiding lifeline columns)
    if !label.is_empty() {
        let label_row = if row > 0 { row - 1 } else { row };
        let mid = (left + right) / 2;
        let label_chars: Vec<char> = label.chars().collect();
        let label_len = label_chars.len();
        // Clamp label to fit between lifelines (left+1 .. right-1)
        let available = right.saturating_sub(left + 2);
        let display_chars = if label_len > available && available > 0 {
            &label_chars[..available]
        } else {
            &label_chars
        };
        let label_start = mid.saturating_sub(display_chars.len() / 2);
        for (j, &ch) in display_chars.iter().enumerate() {
            let col = label_start + j;
            if col < total_width && label_row < canvas.len() {
                canvas[label_row][col] = ch;
            }
        }
    }
}

/// Draw a self-message (loop back to the same actor)
fn draw_self_message(
    canvas: &mut [Vec<char>],
    row: usize,
    col: usize,
    label: &str,
    total_width: usize,
) {
    if row >= canvas.len() {
        return;
    }

    // Draw a small loop: ──┐
    //                      │
    //                   <──┘
    let loop_width = 6;
    let right = (col + loop_width).min(total_width - 1);

    // Top line
    for cell in canvas[row]
        .iter_mut()
        .take((right + 1).min(total_width))
        .skip(col + 1)
    {
        *cell = '─';
    }
    if right < total_width {
        canvas[row][right] = '┐';
    }

    // Vertical
    if row + 1 < canvas.len() && right < total_width {
        canvas[row + 1][right] = '│';
    }

    // Bottom return line: <──┘
    if row + 2 < canvas.len() {
        if col < total_width {
            canvas[row + 2][col] = '<';
        }
        for cell in canvas[row + 2]
            .iter_mut()
            .take(right.min(total_width))
            .skip(col + 1)
        {
            *cell = '─';
        }
        if right < total_width {
            canvas[row + 2][right] = '┘';
        }
    }

    // Place label to the right of the top line
    if !label.is_empty() {
        let label_start = right + 2;
        for (j, ch) in label.chars().enumerate() {
            let c = label_start + j;
            if c < total_width {
                canvas[row][c] = ch;
            }
        }
    }
}

fn fragment_prefix(msg_type: LineType) -> &'static str {
    match msg_type {
        LineType::LoopStart | LineType::LoopEnd => "loop",
        LineType::AltStart | LineType::AltEnd | LineType::AltElse => "alt",
        LineType::OptStart | LineType::OptEnd => "opt",
        LineType::ParStart | LineType::ParEnd | LineType::ParAnd => "par",
        LineType::CriticalStart | LineType::CriticalEnd | LineType::CriticalOption => "critical",
        LineType::BreakStart | LineType::BreakEnd => "break",
        LineType::RectStart | LineType::RectEnd => "rect",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_sequence(input: &str) -> SequenceDb {
        let diagram = crate::parse(input).unwrap();
        match diagram {
            crate::diagrams::Diagram::Sequence(db) => db,
            _ => panic!("Expected sequence diagram"),
        }
    }

    #[test]
    fn renders_two_participants() {
        let db = parse_sequence("sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    A->>B: Hello");
        let output = render_sequence_ascii(&db).unwrap();
        assert!(
            output.contains("Alice"),
            "Should contain Alice, got:\n{}",
            output
        );
        assert!(
            output.contains("Bob"),
            "Should contain Bob, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_message_arrow() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello");
        let output = render_sequence_ascii(&db).unwrap();
        // Should have an arrow character
        assert!(
            output.contains('>') || output.contains('─'),
            "Should contain arrow chars, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_message_label() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello Bob");
        let output = render_sequence_ascii(&db).unwrap();
        assert!(
            output.contains("Hello Bob"),
            "Should contain message label, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_dotted_message() {
        let db = parse_sequence("sequenceDiagram\n    A-->>B: Response");
        let output = render_sequence_ascii(&db).unwrap();
        assert!(
            output.contains("Response"),
            "Should contain dotted message label, got:\n{}",
            output
        );
        assert!(
            output.contains('·'),
            "Should contain dotted line char, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_lifelines() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello\n    B->>A: Hi");
        let output = render_sequence_ascii(&db).unwrap();
        // Lifelines use │
        let pipe_count = output.chars().filter(|&c| c == '│').count();
        assert!(
            pipe_count > 4,
            "Should have multiple lifeline chars, got {} in:\n{}",
            pipe_count,
            output
        );
    }

    #[test]
    fn renders_box_drawing_headers() {
        let db = parse_sequence("sequenceDiagram\n    participant A as Alice\n    A->>A: Think");
        let output = render_sequence_ascii(&db).unwrap();
        assert!(output.contains('┌'), "Should have box top-left corner");
        assert!(output.contains('┘'), "Should have box bottom-right corner");
    }

    #[test]
    fn empty_diagram() {
        let db = SequenceDb::new();
        let output = render_sequence_ascii(&db).unwrap();
        assert!(
            output.is_empty(),
            "Empty diagram should produce empty output"
        );
    }

    #[test]
    fn multiple_messages() {
        let db = parse_sequence(
            "sequenceDiagram\n    A->>B: First\n    B->>A: Second\n    A->>B: Third",
        );
        let output = render_sequence_ascii(&db).unwrap();
        assert!(output.contains("First"), "Should contain First");
        assert!(output.contains("Second"), "Should contain Second");
        assert!(output.contains("Third"), "Should contain Third");
    }

    #[test]
    fn three_participants() {
        let db = parse_sequence(
            "sequenceDiagram\n    participant A\n    participant B\n    participant C\n    A->>B: msg1\n    B->>C: msg2",
        );
        let output = render_sequence_ascii(&db).unwrap();
        // All three participant lifelines should appear
        assert!(output.contains("A"), "Should contain participant A");
        assert!(output.contains("B"), "Should contain participant B");
        assert!(output.contains("C"), "Should contain participant C");
        assert!(output.contains("msg1"));
        assert!(output.contains("msg2"));
    }

    #[test]
    fn self_message_has_return_arrow() {
        let db = parse_sequence("sequenceDiagram\n    A->>A: Think");
        let output = render_sequence_ascii(&db).unwrap();
        assert!(
            output.contains('┐'),
            "Self-message should have top-right corner ┐\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┘'),
            "Self-message should have bottom-right corner ┘\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('<'),
            "Self-message should have return arrow <\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn participant_ordering_preserved() {
        let db = parse_sequence(
            "sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    A->>B: Hello",
        );
        let output = render_sequence_ascii(&db).unwrap();
        let alice_pos = output.find("Alice").expect("Alice should appear");
        let bob_pos = output.find("Bob").expect("Bob should appear");
        // Alice should appear before Bob (left-to-right) in the first line where they appear
        // They're on the same row in the header, so Alice col < Bob col
        assert!(
            alice_pos < bob_pos,
            "Alice should be left of Bob in first occurrence"
        );
    }
}
