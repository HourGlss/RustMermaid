# Selkie Specifications

This file is generated from `@spec` annotations. Do not edit it manually.

## FLOW

### FLOW-1.1

When a flowchart declares a named subgraph, the application shall preserve the subgraph title text in the parsed model.

Source: `src/diagrams/flowchart/parser.rs:733`

### FLOW-1.2

When a subgraph declares its own direction, the application shall preserve that direction without changing the parent flowchart direction.

Source: `src/render/flowchart.rs:392`

### FLOW-1.3

When a flowchart contains subgraph member nodes, the application shall preserve parent-child relationships in the layout graph.

Source: `src/render/flowchart.rs:187`

### FLOW-2.1

When a flowchart edge has a Mermaid label, the application shall preserve that label in the layout graph edge model.

Source: `src/render/flowchart.rs:259`

### FLOW-2.2

When an ASCII flowchart edge label is placed near a diamond node, the application shall render the full edge label text without truncation.

Source: `tests/flowchart_edge_label_truncation.rs:3`

### FLOW-3.1

When a flowchart edge is rendered to SVG, the application shall emit an SVG path for the edge route.

Source: `src/render/flowchart.rs:352`

### FLOW-4.1

When a Mermaid flowchart applies a class to nodes, the application shall preserve the class assignment in the parsed node model.

Source: `src/diagrams/flowchart/parser.rs:1861`
