#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use js_sys::{Function, Reflect};
use wasm_bindgen::JsValue;

use selkie::wasm::{
    apply_graph_patch_result_json, graph_to_mermaid_text, initialize, layout_graph_json, parse,
    parse_to_graph_json, render, render_graph_json, render_graph_parts_json, render_text,
    route_edges_for_node_json,
};

#[test]
fn render_text_returns_svg() {
    let svg = render_text("flowchart TD; A-->B;").expect("render_text should succeed");
    assert!(svg.contains("<svg"), "expected SVG output");
}

#[test]
fn parse_validates_input() {
    parse("flowchart TD; A-->B;").expect("parse should succeed");
}

#[test]
fn render_matches_mermaid_shape() {
    initialize(JsValue::NULL);
    let value = render("diagram1", "flowchart TD; A-->B;").expect("render should succeed");
    let svg = Reflect::get(&value, &JsValue::from_str("svg"))
        .expect("should get svg")
        .as_string()
        .expect("svg should be a string");
    assert!(svg.contains("<svg"), "expected SVG output");

    let id = Reflect::get(&value, &JsValue::from_str("id"))
        .expect("should get id")
        .as_string()
        .expect("id should be a string");
    assert_eq!(id, "diagram1");

    let bind_functions = Reflect::get(&value, &JsValue::from_str("bindFunctions"))
        .expect("should get bindFunctions");
    assert!(
        bind_functions.is_function(),
        "bindFunctions should be a function"
    );
    let function: Function = bind_functions
        .dyn_into()
        .expect("bindFunctions should be callable");
    let _ = function;
}

#[test]
fn graph_json_api_round_trips_flowchart() {
    let graph_json = parse_to_graph_json(
        r#"flowchart TD
  A[Start] --> B{Decision}
  B -->|Yes| C[Done]
"#,
    )
    .expect("parse_to_graph_json should succeed");
    let graph: serde_json::Value =
        serde_json::from_str(&graph_json).expect("graph json should parse");

    assert_eq!(graph["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(graph["edges"].as_array().unwrap().len(), 2);
    assert_eq!(graph["nodes"][1]["shape"], "diamond");
    assert_eq!(graph["edges"][1]["label"], "Yes");

    let text = graph_to_mermaid_text(&graph_json).expect("graph_to_mermaid_text should succeed");
    assert!(text.contains("flowchart TD"));
    assert!(text.contains("B -->|Yes| C"));

    let svg = render_graph_json(&graph_json).expect("render_graph_json should succeed");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Decision"));

    let laid_out = layout_graph_json(&graph_json).expect("layout_graph_json should succeed");
    let laid_out_graph: serde_json::Value =
        serde_json::from_str(&laid_out).expect("layout graph json should parse");
    assert!(
        laid_out_graph["nodes"][0]["position"].is_object(),
        "layout should add node positions"
    );

    let parts =
        render_graph_parts_json(&graph_json).expect("render_graph_parts_json should succeed");
    let parts_json: serde_json::Value =
        serde_json::from_str(&parts).expect("render parts json should parse");
    assert_eq!(parts_json["nodes"].as_array().unwrap().len(), 3);

    let routes = route_edges_for_node_json(&graph_json, "B")
        .expect("route_edges_for_node_json should succeed");
    let routes_json: serde_json::Value =
        serde_json::from_str(&routes).expect("routes json should parse");
    assert_eq!(routes_json["node_id"], "B");

    let patch = r#"{"op":"move_node","id":"A","x":50.0,"y":75.0,"locked":true}"#;
    let patch_result = apply_graph_patch_result_json(&graph_json, patch)
        .expect("apply_graph_patch_result_json should succeed");
    let patch_json: serde_json::Value =
        serde_json::from_str(&patch_result).expect("patch result json should parse");
    assert!(patch_json["affected_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "node:A"));
}
