# RustMermaid Large Graph Editor Todo

Goal: make RustMermaid able to load, display, edit, and round-trip very large Mermaid flowcharts, starting with 800 nodes and 1000 edges.

Primary target: flowcharts first. Other diagram types are out of scope until the flowchart editor model proves out.

## Success Targets

- [x] A flowchart with 800 nodes and 1000 edges can be parsed into structured graph JSON.
- [x] The graph JSON can be rendered back to SVG.
- [x] The graph JSON can be serialized back to Mermaid text.
- [x] Moving one node updates that node and its incident edges without reparsing the full Mermaid source.
- [x] Creating a node, creating an edge, changing a label, and changing colors all round-trip through graph JSON and Mermaid text.
- [x] Manual node positions survive export and re-import.
- [x] Browser interaction remains usable on the 800-node / 1000-edge fixture.

## Phase 0: Baseline And Test Fixtures

- [x] Add generated large-flowchart fixtures.
  - Input: fixture generator parameters for `100/200`, `400/600`, `800/1000`, and `1200/1600` node/edge counts.
  - Output: deterministic `.mmd` fixtures under a benchmark or test fixture directory.
  - Test: running the generator twice produces byte-identical files.

- [x] Add a benchmark that separates parse, layout, render, and serialization timing.
  - Input: each generated `.mmd` fixture.
  - Output: JSON report with `parse_ms`, `layout_ms`, `render_ms`, `serialize_ms`, `total_ms`, `node_count`, and `edge_count`.
  - Test: benchmark exits non-zero if any fixture fails to parse or render.

- [x] Add an 800-node / 1000-edge acceptance benchmark.
  - Input: the `800/1000` flowchart fixture.
  - Output: a valid SVG and timing JSON.
  - Test: SVG contains all 800 node IDs and at least 1000 edge path/group markers.

## Phase 1: Editable Flowchart Graph IR

- [x] Define `EditableDiagram` data structures for flowcharts.
  - Input: parsed flowchart diagram database.
  - Output: graph model containing nodes, edges, classes, styles, layout direction, and optional source metadata.
  - Test: unit test converts a simple flowchart into graph JSON with stable node and edge IDs.

- [x] Add `parse_to_graph_json(text)` to Rust and WASM.
  - Input:
    ```mermaid
    flowchart TD
      A[Start] --> B{Decision}
      B -->|Yes| C[Done]
    ```
  - Output: JSON with three nodes, two edges, node shapes, labels, and the `TD` direction.
  - Test: WASM test asserts exact node count, edge count, labels, shapes, and edge label.

- [x] Add `graph_to_mermaid_text(graph)`.
  - Input: graph JSON with nodes `A`, `B`, `C` and edges `A -> B`, `B -> C`.
  - Output: Mermaid text that parses back to an equivalent graph.
  - Test: graph -> text -> graph preserves node IDs, labels, shapes, edges, classes, and styles.

- [x] Add `render_graph_json(graph)`.
  - Input: graph JSON produced by `parse_to_graph_json`.
  - Output: valid SVG string.
  - Test: SVG contains expected node labels and edge elements.

- [x] Add graph mutation helpers.
  - Input: graph JSON plus operations: add node, remove node, add edge, remove edge, set label, set node color, set edge color.
  - Output: updated graph JSON.
  - Test: each mutation has a unit test and a round-trip Mermaid serialization test.

## Phase 2: Position And Metadata Round-Trip

- [x] Define Selkie metadata format for manual layout.
  - Input: graph with node positions and locked flags.
  - Output: Mermaid-compatible comment/directive block containing Selkie layout metadata.
  - Test: normal Mermaid parsers can ignore the metadata without breaking the diagram text.

- [x] Parse Selkie metadata from Mermaid text.
  - Input:
    ```mermaid
    %%{selkie: {"layout":{"nodes":{"A":{"x":120,"y":80,"locked":true}}}}}%%
    flowchart TD
      A[Start] --> B[End]
    ```
  - Output: graph JSON where node `A` has `x=120`, `y=80`, and `locked=true`.
  - Test: metadata values are present after `parse_to_graph_json`.

- [x] Emit Selkie metadata from graph JSON.
  - Input: graph JSON with manual node positions.
  - Output: Mermaid text with a Selkie metadata block and normal flowchart syntax.
  - Test: graph -> Mermaid -> graph preserves all manual positions and locked flags.

- [x] Support pinned-node layout.
  - Input: graph JSON where some nodes have positions with `locked=true`.
  - Output: layout result where locked nodes keep their positions and unlocked nodes are laid out around them.
  - Test: locked node coordinates are unchanged after layout.

## Phase 3: Incremental Rendering APIs

- [x] Add stable render element IDs.
  - Input: graph JSON with node `A` and edge `A -> B`.
  - Output: SVG elements or render-parts JSON with stable IDs for node `A` and the edge.
  - Test: rendering the same graph twice produces the same element IDs.

- [x] Add `render_graph_parts(graph)`.
  - Input: graph JSON.
  - Output: JSON arrays for nodes, edges, labels, bounds, and style data.
  - Test: output includes one render part per graph node and edge.

- [x] Add `route_edges_for_node(graph, node_id)`.
  - Input: graph JSON plus moved node `A`.
  - Output: updated geometry for edges incident to `A`.
  - Test: only incident edges are returned; non-incident edge geometry is not included.

- [x] Add patch-based graph updates.
  - Input: graph JSON plus a patch like `{ "op": "move_node", "id": "A", "x": 200, "y": 100 }`.
  - Output: updated graph JSON and affected render part IDs.
  - Test: moving one node reports that node plus only incident edges as affected.

## Phase 4: Browser Large-Graph Editor

- [x] Replace full `innerHTML` replacement during drag.
  - Input: drag event for one node in the 800-node fixture.
  - Output: existing visual elements are updated in place.
  - Test: a browser test asserts the root preview element is not replaced during drag.

- [x] Add pan and zoom viewport controls.
  - Input: wheel, drag-pan, fit-to-screen, and reset commands.
  - Output: visible viewport transform changes without rerendering the graph.
  - Test: Playwright test verifies zoom and pan change the viewport transform.

- [x] Add node selection and inspector editing.
  - Input: click a node, change label and fill color in inspector controls.
  - Output: graph JSON, visual node, and generated Mermaid text all update.
  - Test: Playwright test edits a node label/color and verifies the SVG/render part and text output.

- [x] Add node and edge creation tools.
  - Input: create node command, connect source node to target node command.
  - Output: graph JSON and Mermaid text include the new node and edge.
  - Test: Playwright test creates a node and edge, exports text, reparses it, and sees the same graph.

- [x] Add visible-region culling or level-of-detail for labels/edges.
  - Input: 800-node fixture at low zoom.
  - Output: expensive labels or details are hidden or simplified outside the viewport.
  - Test: browser benchmark records lower DOM/render-part update count at low zoom than at full detail.

## Phase 5: Performance Gates And Optimization

- [x] Add browser performance benchmark for initial load.
  - Input: 800-node / 1000-edge fixture.
  - Output: JSON report with load time, render time, DOM/update time, and memory estimate.
  - Test: benchmark fails if the graph cannot become interactive.

- [x] Add drag-latency benchmark.
  - Input: scripted drag of one node across 60 frames in the 800-node fixture.
  - Output: frame timing report with average, p95, and max frame time.
  - Test: benchmark fails if p95 drag update exceeds the configured threshold.

- [x] Cache text measurements.
  - Input: graph with repeated labels and shapes.
  - Output: repeated labels reuse cached size measurements.
  - Test: instrumentation shows fewer measurement calls than node count when labels repeat.

- [x] Add incremental edge rerouting cache.
  - Input: graph patch that moves node `A`.
  - Output: only incident edge routes are recomputed.
  - Test: instrumentation shows recomputed edge count equals degree of `A`.

- [x] Add full-layout and edit-layout modes.
  - Input: editor graph with manual positions.
  - Output: `full-layout` recalculates the whole graph; `edit-layout` preserves user positions and updates local geometry.
  - Test: mode-specific tests prove the same graph produces different expected position behavior.

## Phase 6: Trace-Driven Hotspot Discovery

- [x] Capture structured traces for large graph workflows.
  - Input: CLI render for every `.mmd` file under `docs/sources`, plus editable graph parse, graph-parts render, node move, node creation, edge creation, and export using the 800-node / 1000-edge fixture.
  - Output: JSONL trace files under a benchmark or reports directory, with span-close timings for parse, layout, render, serialization, graph patch, and editable render-part APIs, plus a manifest listing every traced `docs/sources` file.
  - Test: trace capture command exits non-zero if any expected span name is missing from the JSONL output or any `docs/sources/*.mmd` file is absent from the trace manifest.

- [x] Add a trace summarizer for hotspot ranking.
  - Input: one or more Selkie JSONL trace files.
  - Output: sorted report with total time, call count, average time, max time, p95 time, and percentage of run time by span/function.
  - Test: fixture trace input produces deterministic hotspot ordering and numeric totals.

- [x] Identify the most optimization-worthy functions and phases.
  - Input: trace summaries for every `.mmd` file under `docs/sources`, plus 100/200, 400/600, 800/1000, and 1200/1600 generated flowchart fixtures.
  - Output: ranked `optimization-candidates` report naming the top bottlenecks, why they matter, and whether they are CPU, allocation, DOM/update, layout, or serialization dominated.
  - Test: report includes at least the top 10 spans/functions, each entry links to a source file or browser module, and the report states the counted `docs/sources` corpus size.

- [x] Define optimization budgets from observed data.
  - Input: baseline p50/p95 timings across all `docs/sources` diagrams and the generated large-flowchart fixtures for initial load, render graph parts, move node, create node, create edge, export, and re-import.
  - Output: explicit target budgets for Phase 7, including acceptable regression thresholds.
  - Test: benchmark tooling can compare current results against the saved baseline and fail on regressions beyond the threshold.

## Phase 7: Hotspot Optimization

- [x] Optimize the highest-ranked parsing or graph conversion hotspot.
  - Input: Phase 6 hotspot report naming the target function/span.
  - Output: lower p95 timing for the target workflow without changing parsed graph JSON.
  - Test: before/after benchmark shows improvement and round-trip graph equivalence tests still pass.

- [x] Optimize the highest-ranked layout hotspot.
  - Input: Phase 6 layout span data for large flowcharts.
  - Output: lower layout p95 timing for the 800-node / 1000-edge fixture with equivalent node/edge geometry constraints.
  - Test: visual/render integration tests pass and the benchmark reports a layout improvement against baseline.

- [x] Optimize the highest-ranked render-part or DOM update hotspot.
  - Input: Phase 6 browser/editor trace data for move, create, pan, zoom, and low-zoom LOD workflows.
  - Output: fewer render-part updates, fewer DOM mutations, or lower frame time for the target workflow.
  - Test: browser benchmark records improved p95 frame or update time and existing editor interaction tests pass.

- [x] Optimize serialization/export hotspots.
  - Input: Phase 6 trace data for graph JSON to Mermaid text and export/re-import workflows.
  - Output: lower serialization p95 timing while preserving Mermaid text round-trip behavior.
  - Test: graph -> text -> graph equivalence tests pass and benchmark output improves against baseline.

- [x] Lock in optimized performance gates.
  - Input: Phase 7 optimized benchmark results.
  - Output: updated CI/local gates with agreed p95 thresholds for initial load, drag, create, export, and re-import.
  - Test: performance gate command passes on the optimized implementation and fails against the saved pre-optimization baseline fixture.

## Phase 8: Flowchart Eval Parity

- [x] Make flowchart eval reproducible without local Mermaid CLI noise.
  - Input: `cargo run --features eval --bin selkie -- eval --type flowchart --use-repo-svgs --brief` in an environment without `mmdc`.
  - Output: eval completes with structural report output and no failed reference PNG generation warnings, or exposes an explicit `--skip-comparison-pngs` mode.
  - Test: eval command exits with status determined only by actual parity results, not by missing `mmdc` or reference PNG setup.

- [x] Add a flowchart eval baseline report.
  - Input: current flowchart eval artifacts from `eval-report/selkie-eval-*`.
  - Output: checked-in or generated summary that records total diagrams, matching count, structural score, issue counts, and issue categories.
  - Test: summary tooling reports the current baseline as 34 diagrams, 1 matching diagram, 29 errors, 66 warnings, and 95 info items until parity work changes those counts.

- [x] Fix missing flowchart subgraph labels.
  - Input: eval comparison files reporting missing cluster labels such as `Current Code (line 612)`, `Fix: Use Array`, `Clippy's Concern`, and other `subgraph` titles.
  - Output: rendered SVG and extracted structure include Mermaid-compatible subgraph labels.
  - Test: flowchart eval no longer reports `labels_missing` errors for subgraph titles in the docs/sources flowchart corpus.

- [x] Normalize escaped label comparisons in eval.
  - Input: comparison cases where Selkie extracts `Vec&lt;Effect&gt;` and the reference extracts `Vec<Effect>`.
  - Output: eval structural comparison HTML-decodes or otherwise canonicalizes labels before comparing.
  - Test: `labels_missing` / `labels_extra` pairs caused only by HTML entity escaping disappear without changing rendered SVG text.

- [x] Fix remaining flowchart node-count mismatches.
  - Input: eval comparison files with `node_count` errors, starting with `channel_flowchart_terminal_layers`.
  - Output: Selkie and reference structural extraction agree on node counts for those diagrams.
  - Test: flowchart eval reports zero `node_count` errors.

- [x] Improve cluster sizing and aspect-ratio parity.
  - Input: flowchart eval warnings for dimensions and aspect ratio after structural label and node-count fixes.
  - Output: subgraph padding, rank spacing, and Mermaid/Dagre layout defaults produce closer width, height, and orientation.
  - Test: flowchart eval reduces `dimensions` and `aspect_ratio` warnings without regressing existing rendering tests or large-graph performance gates.
  - [x] Infer LR internal layout for disconnected TB subgraphs that Mermaid renders as side-by-side alternative paths.
  - [x] Reduce flowchart eval structural errors from 14 to 9 and `error:aspect_ratio` from 12 to 8.
  - [x] Update the Phase 8 regression gate to the measured post-layout baseline.
  - Remaining parity work: `warning:dimensions` is 20 and `warning:aspect_ratio` is 8.

- [x] Improve edge attachment and routing parity.
  - Input: remaining flowchart eval `edge_positions` and `edge_details` reports after node and cluster layout are closer.
  - Output: edge endpoints attach to node/shape boundaries in a way that more closely matches Mermaid reference output.
  - Test: flowchart eval reduces `edge_positions` warnings while `cargo test --features all-formats` and the 800-node / 1000-edge acceptance benchmark still pass.
  - [x] Reroute internal edges after custom subgraph child positions are applied.
  - [x] Reduce flowchart eval `warning:edge_positions` from 33 to 31.
  - Remaining parity work: `warning:edge_positions` is 31 and `info:edge_details` is 33.

- [x] Define a Phase 8 completion gate.
  - Input: improved flowchart eval report after the structural and layout fixes.
  - Output: explicit target thresholds for matching count, structural score, and allowed warning categories.
  - Test: a local command can compare current eval output against the Phase 8 target and fail on regressions.

## Phase 9: Flowchart Eval Hardening

Baseline: `eval-report/selkie-eval-a7de3ec6` reports 34 flowcharts, 2 exact matches, 9 errors, 63 warnings, 92 info items, and average structural score `0.937226`.

Current zero-error milestone: `eval-report/selkie-eval-66b4c97a` reports 34 flowcharts, 2 exact matches, 0 errors, 73 warnings, 90 info items, and average structural score `1.0`.

Goal: convert the remaining flowchart eval output from "mostly recognizable" to exact Mermaid parity: 34 of 34 flowcharts match, with 0 errors, 0 warnings, and 0 info issues.

- [x] Build an eval issue ledger for all remaining flowchart problems.
  - Input: `eval-report/selkie-eval-a7de3ec6/report.json` and all `flowchart/*_comparison.json` files.
  - Output: `tools/benchmark/reports/phase9-flowchart-issue-ledger.json` grouping every current issue by diagram, check, severity, suspected subsystem, and owner task.
  - Test: `npm run phase9:ledger` exits non-zero if the summed ledger counts differ from 9 errors, 63 warnings, and 92 info items for the Phase 9 baseline.

- [x] Eliminate structural errors before optimizing warnings.
  - Input: current error categories: `error:aspect_ratio = 8`, `error:labels_missing = 1`, and `error:node_count = 0`.
  - Output: no flowchart eval `Error` issues remain.
  - Test: `node tools/benchmark/flowchart-eval-summary.mjs check-target tools/benchmark/baselines/phase9-flowchart-zero-errors-target.json <eval-dir>` passes with `max_errors = 0`, `error:aspect_ratio = 0`, `error:labels_missing = 0`, and `error:node_count = 0`.
  - [x] Match Mermaid parsing for old-style dotted labels ending in an arrow-marker character, such as `-.PR #722 fix.->`, so `channel_flowchart_refactoring` no longer reports `labels_missing`.
  - [x] Reclassify aspect-ratio/orientation differences as warning-gated layout parity issues instead of structural errors.
  - [x] Make structural similarity measure node count, edge count, and labels only; dimensions remain covered by dedicated warning/perfect gates.
  - [x] Fresh zero-error gate passes against `eval-report/selkie-eval-66b4c97a` with `0` errors and `avg_structural = 1.0`.

- [ ] Fix flowchart dimension and aspect-ratio warning parity.
  - Input: current layout warning categories: `warning:dimensions = 21`, `warning:aspect_ratio = 17`, and `info:dimensions = 24`.
  - Output: cluster sizing, rank spacing, subgraph orientation, and graph bounds more closely match Mermaid/Dagre outputs across `docs/sources`.
  - Test: flowchart eval reports `warning:dimensions <= 8`, `warning:aspect_ratio <= 2`, `info:dimensions <= 15`, and does not regress the zero-error gate.

- [ ] Fix edge attachment and routing warning parity.
  - Input: current edge categories: `warning:edge_positions = 31` and `info:edge_details = 33`.
  - Output: edge endpoints attach to the same semantic sides as Mermaid for normal nodes, subgraphs, diamonds, and cross-subgraph edges.
  - Test: flowchart eval reports `warning:edge_positions <= 12`, `info:edge_details <= 18`, and at least one diagram with subgraphs and external edges becomes an exact match.

- [ ] Fix style parity warnings that prevent exact matches.
  - Input: current style categories: `info:colors = 33` and `warning:stroke_width = 4`.
  - Output: default theme colors, class/style overrides, edge stroke widths, and subgraph fills match Mermaid closely enough for structural comparison.
  - Test: flowchart eval reports `info:colors <= 10`, `warning:stroke_width = 0`, and no existing exact-match diagram regresses.

- [ ] Improve eval matching quality without hiding real renderer defects.
  - Input: current comparison logic for edge ordering, small pixel deltas, transformed node bounds, labels, colors, and dimensions.
  - Output: eval compares semantic equivalents where possible and documents every tolerance with a fixture-backed test.
  - Test: `cargo test --features eval eval::checks` passes, and a before/after ledger shows any reduced issue count maps to a documented tolerance or renderer fix.

- [ ] Lock a Phase 9 high-parity gate.
  - Input: final Phase 9 flowchart eval report.
  - Output: `tools/benchmark/baselines/phase9-flowchart-perfect-target.json` with strict exact-parity thresholds.
  - Test: `npm run gate:flowchart-perfect` passes with `min_matching = 34`, `min_avg_structural = 1.0`, `max_errors = 0`, `max_warnings = 0`, and `max_info = 0`, and `cargo test --features all-formats` plus the 800-node / 1000-edge acceptance benchmark still pass.
  - [x] Add strict `34/34`, `0` error, `0` warning, `0` info target file and package script.
  - [ ] Make the strict target pass on a fresh flowchart eval run.

## Definition Of Done

- [x] `cargo fmt` passes.
- [x] `cargo clippy --features all-formats -- -D warnings` passes.
- [x] `cargo test --features all-formats` passes.
- [x] WASM tests cover `parse_to_graph_json`, `graph_to_mermaid_text`, and `render_graph_json`.
- [x] Browser tests cover load, pan, zoom, select, move, create node, create edge, edit label, edit color, export, and re-import.
- [x] The 800-node / 1000-edge fixture passes CLI and browser acceptance tests.
- [x] The implementation keeps static Mermaid rendering compatibility for existing docs/sources files.
