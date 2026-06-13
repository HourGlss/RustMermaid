use js_sys::{Function, Object, Reflect};
use wasm_bindgen::prelude::*;

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

/// Parse Mermaid flowchart text to editable graph JSON.
#[wasm_bindgen]
pub fn parse_to_graph_json(input: &str) -> Result<String, JsValue> {
    crate::editable::parse_to_graph_json(input).map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Serialize editable graph JSON back to Mermaid text.
#[wasm_bindgen]
pub fn graph_to_mermaid_text(graph_json: &str) -> Result<String, JsValue> {
    crate::editable::graph_to_mermaid_text_json(graph_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Render editable graph JSON to SVG.
#[wasm_bindgen]
pub fn render_graph_json(graph_json: &str) -> Result<String, JsValue> {
    crate::editable::render_graph_json(graph_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Apply layout to editable graph JSON and return positioned graph JSON.
#[wasm_bindgen]
pub fn layout_graph_json(graph_json: &str) -> Result<String, JsValue> {
    crate::editable::layout_graph_json(graph_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Return editable render parts with stable IDs.
#[wasm_bindgen]
pub fn render_graph_parts_json(graph_json: &str) -> Result<String, JsValue> {
    crate::editable::render_graph_parts_json(graph_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Return editable render parts using either full layout or cached edit layout.
#[wasm_bindgen]
pub fn render_graph_parts_with_layout_mode_json(
    graph_json: &str,
    layout_mode: &str,
) -> Result<String, JsValue> {
    crate::editable::render_graph_parts_with_layout_mode_json(graph_json, layout_mode)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Return edge routes incident to one node.
#[wasm_bindgen]
pub fn route_edges_for_node_json(graph_json: &str, node_id: &str) -> Result<String, JsValue> {
    crate::editable::route_edges_for_node_json(graph_json, node_id)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Return edge routes incident to one node using full or cached edit layout.
#[wasm_bindgen]
pub fn route_edges_for_node_with_layout_mode_json(
    graph_json: &str,
    node_id: &str,
    layout_mode: &str,
) -> Result<String, JsValue> {
    crate::editable::route_edges_for_node_with_layout_mode_json(graph_json, node_id, layout_mode)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Apply a JSON patch operation to editable graph JSON.
#[wasm_bindgen]
pub fn apply_graph_patch_json(graph_json: &str, patch_json: &str) -> Result<String, JsValue> {
    crate::editable::apply_graph_patch_json(graph_json, patch_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Apply a JSON patch and return the updated graph plus affected render IDs.
#[wasm_bindgen]
pub fn apply_graph_patch_result_json(
    graph_json: &str,
    patch_json: &str,
) -> Result<String, JsValue> {
    crate::editable::apply_graph_patch_result_json(graph_json, patch_json)
        .map_err(|err| JsValue::from_str(&err.to_string()))
}
