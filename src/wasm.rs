use js_sys::{Function, Object, Reflect};
use wasm_bindgen::prelude::*;

use crate::diagrams::positions::{
    detect_positions, encode_positions as encode_pos, update_positions, NodePosition,
    PositionOverrides,
};

/// Mirror mermaid-js's initialize API (currently a no-op).
#[wasm_bindgen]
pub fn initialize(_config: JsValue) {}

/// Validate a Mermaid diagram and return an error on failure.
#[wasm_bindgen]
pub fn parse(input: &str) -> Result<(), JsValue> {
    crate::parse(input)
        .map(|_| ())
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Render Mermaid diagram text to SVG with a mermaid-js compatible return shape.
#[wasm_bindgen]
pub fn render(id: &str, input: &str) -> Result<JsValue, JsValue> {
    let svg =
        crate::render::render_text(input).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let result = Object::new();
    Reflect::set(&result, &JsValue::from_str("id"), &JsValue::from_str(id))?;
    Reflect::set(&result, &JsValue::from_str("svg"), &JsValue::from_str(&svg))?;
    let bind_functions = Function::new_no_args("");
    Reflect::set(
        &result,
        &JsValue::from_str("bindFunctions"),
        &bind_functions.into(),
    )?;
    Ok(result.into())
}

/// Render Mermaid diagram text to SVG (WASM-friendly).
#[wasm_bindgen]
pub fn render_text(input: &str) -> Result<String, JsValue> {
    crate::render::render_text(input).map_err(|err| JsValue::from_str(&err.to_string()))
}

// =============================================================================
// Draggable Nodes API
// =============================================================================

/// Update a single node's position in the mermaid source text.
///
/// This function:
/// 1. Parses existing position overrides from the source
/// 2. Updates (or adds) the position for the specified node
/// 3. Returns the updated source text with new position comment
///
/// # Arguments
/// * `source` - The mermaid diagram source text
/// * `node_id` - The ID of the node to update
/// * `x` - The new X coordinate
/// * `y` - The new Y coordinate
///
/// # Returns
/// The updated mermaid source text with the position override
#[wasm_bindgen]
pub fn update_node_position(source: &str, node_id: &str, x: f64, y: f64) -> String {
    // Get existing overrides or create new
    let mut overrides = detect_positions(source).unwrap_or_default();

    // Update the position
    overrides.set(node_id, x, y);

    // Return updated source
    update_positions(source, &overrides)
}

/// Remove a node's position override from the mermaid source text.
///
/// # Arguments
/// * `source` - The mermaid diagram source text
/// * `node_id` - The ID of the node to remove position override for
///
/// # Returns
/// The updated mermaid source text with the position override removed
#[wasm_bindgen]
pub fn remove_node_position(source: &str, node_id: &str) -> String {
    let mut overrides = detect_positions(source).unwrap_or_default();
    overrides.remove(node_id);
    update_positions(source, &overrides)
}

/// Clear all position overrides from the mermaid source text.
///
/// # Arguments
/// * `source` - The mermaid diagram source text
///
/// # Returns
/// The mermaid source text with all position overrides removed
#[wasm_bindgen]
pub fn clear_positions(source: &str) -> String {
    update_positions(source, &PositionOverrides::new())
}

/// Get all position overrides from mermaid source text as a JSON string.
///
/// # Arguments
/// * `source` - The mermaid diagram source text
///
/// # Returns
/// A JSON string containing the position overrides, e.g.:
/// `{"A": {"x": 100, "y": 200}, "B": {"x": 300, "y": 400}}`
/// Returns "{}" if no positions are found.
#[wasm_bindgen]
pub fn get_positions_json(source: &str) -> String {
    match detect_positions(source) {
        Some(overrides) => {
            serde_json::to_string(&overrides.positions).unwrap_or_else(|_| "{}".to_string())
        }
        None => "{}".to_string(),
    }
}

/// Set multiple node positions at once from a JSON string.
///
/// # Arguments
/// * `source` - The mermaid diagram source text
/// * `positions_json` - A JSON string with positions, e.g.:
///   `{"A": {"x": 100, "y": 200}, "B": {"x": 300, "y": 400}}`
///
/// # Returns
/// The updated mermaid source text with the position overrides,
/// or the original source if the JSON is invalid.
#[wasm_bindgen]
pub fn set_positions_json(source: &str, positions_json: &str) -> String {
    match serde_json::from_str::<std::collections::HashMap<String, NodePosition>>(positions_json) {
        Ok(positions) => {
            let overrides = PositionOverrides { positions };
            update_positions(source, &overrides)
        }
        Err(_) => source.to_string(),
    }
}

/// Encode a positions comment string from a JSON object.
///
/// # Arguments
/// * `positions_json` - A JSON string with positions
///
/// # Returns
/// The positions comment string, e.g.:
/// `%% selkie:positions {"A": {"x": 100, "y": 200}} %%`
#[wasm_bindgen]
pub fn encode_positions_comment(positions_json: &str) -> String {
    match serde_json::from_str::<std::collections::HashMap<String, NodePosition>>(positions_json) {
        Ok(positions) => {
            let overrides = PositionOverrides { positions };
            encode_pos(&overrides)
        }
        Err(_) => String::new(),
    }
}
