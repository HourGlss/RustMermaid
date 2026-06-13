#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createReadStream, readFileSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(__dirname, '../..');
const largeFixturePath = resolve(
  repoRoot,
  'tools/benchmark/fixtures/flowchart-800-nodes-1000-edges.mmd',
);

const contentTypes = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml; charset=utf-8',
};

function fixtureHtml() {
  const { nodes, edges, edgeLabels, renderParts } = largeFlowchartSvgParts();
  return `<!DOCTYPE html>
  <html>
    <body>
      <div id="preview-container">
        <div id="preview">
          <svg id="diagram-root" viewBox="0 0 4000 3000">
            ${edges}
            ${edgeLabels}
            ${nodes}
          </svg>
        </div>
      </div>
      <button id="zoom-reset"></button>
      <input id="inspector-label" type="text">
      <input id="inspector-color" type="color" value="#ffffff">
      <textarea id="editor"></textarea>
      <script>window.__largeRenderParts = ${JSON.stringify(renderParts)};</script>
    </body>
  </html>`;
}

function largeFlowchartSvgParts() {
  const source = readFileSync(largeFixturePath, 'utf8');
  const nodeIds = [...source.matchAll(/^\s*(N\d+)\[/gm)].map((match) => match[1]);
  const edgeMatches = [...source.matchAll(/^\s*(N\d+)\s*-->\s*(N\d+)/gm)];
  const edgeIds = edgeMatches.map((_, index) => `edge-${index}`);

  assert.equal(nodeIds.length, 800, 'large fixture should contain 800 nodes');
  assert.equal(edgeIds.length, 1000, 'large fixture should contain 1000 edges');

  const nodeParts = nodeIds.map((id, index) => ({
    id: `node:${id}`,
    node_id: id,
    label: id,
    bounds: nodeBoundsForIndex(index),
    classes: [],
    styles: [],
  }));
  const edgeParts = edgeMatches.map((match, index) => ({
    id: `edge:${index}`,
    edge_id: String(index),
    source: match[1],
    target: match[2],
    points: [],
    styles: [],
  }));

  return {
    nodes: nodeIds
      .map((id, index) => {
        const { x, y } = nodeBoundsForIndex(index);
        return `<g id="node-${id}" transform="translate(${x} ${y})"><rect width="80" height="40" fill="#ffffff"></rect><text>${id}</text></g>`;
      })
      .join('\n'),
    edges: edgeIds
      .map((id) => `<g id="${id}"><path d="M 0 0 L 100 100"></path></g>`)
      .join('\n'),
    edgeLabels: edgeParts
      .map((edge) => `<g id="edge-label-${edge.edge_id}"><text>${edge.edge_id}</text></g>`)
      .join('\n'),
    renderParts: {
      nodes: nodeParts,
      edges: edgeParts,
      labels: [],
      bounds: { x: 0, y: 0, width: 4000, height: 3000 },
    },
  };
}

function nodeBoundsForIndex(index) {
  return {
    x: (index % 40) * 95,
    y: Math.floor(index / 40) * 95,
    width: 80,
    height: 40,
  };
}

async function serveRepo() {
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://127.0.0.1');
      if (url.pathname === '/__editor_test__.html') {
        response.writeHead(200, { 'content-type': contentTypes['.html'] });
        response.end(fixtureHtml());
        return;
      }

      if (url.pathname === '/playground/pkg/selkie.js') {
        response.writeHead(200, { 'content-type': contentTypes['.js'] });
        response.end(fakeSelkieModule());
        return;
      }

      const relativePath = decodeURIComponent(url.pathname.replace(/^\/+/, ''));
      const filePath = resolve(repoRoot, relativePath);
      if (!filePath.startsWith(repoRoot)) {
        response.writeHead(403);
        response.end('Forbidden');
        return;
      }

      const fileStat = await stat(filePath);
      if (!fileStat.isFile()) {
        response.writeHead(404);
        response.end('Not found');
        return;
      }

      response.writeHead(200, {
        'content-type': contentTypes[extname(filePath)] ?? 'application/octet-stream',
      });
      createReadStream(filePath).pipe(response);
    } catch {
      response.writeHead(404);
      response.end('Not found');
    }
  });

  await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
  const address = server.address();
  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    close: () => new Promise((resolveClose) => server.close(resolveClose)),
  };
}

function fakeSelkieModule() {
  return String.raw`
export default async function initWasm() {}
export function initialize() {}
export function parse(input) {
  parseToGraph(input);
}
export function render(id, input) {
  return {
    id,
    svg: renderSvg(parseToGraph(input)),
    bindFunctions() {},
  };
}
export function render_text(input) {
  return render('diagram', input).svg;
}
export function parse_to_graph_json(input) {
  return JSON.stringify(parseToGraph(input));
}
export function graph_to_mermaid_text(graphJson) {
  const graph = JSON.parse(graphJson);
  const lines = [${JSON.stringify('flowchart TB')}];
  for (const node of graph.nodes) {
    lines.push('  ' + node.id + '["' + escapeLabel(node.label) + '"]');
  }
  for (const edge of graph.edges) {
    const edgeId = edge.id ? edge.id + '@' : '';
    lines.push('  ' + edge.source + ' ' + edgeId + '--> ' + edge.target);
  }
  for (const node of graph.nodes) {
    if (node.styles?.length) {
      lines.push('  style ' + node.id + ' ' + node.styles.join(','));
    }
  }
  return lines.join('\n');
}
export function render_graph_parts_json(graphJson) {
  const graph = JSON.parse(graphJson);
  return JSON.stringify({
    nodes: graph.nodes.map((node, index) => ({
      id: 'node:' + node.id,
      node_id: node.id,
      label: node.label,
      shape: node.shape || 'square',
      bounds: nodeBounds(index),
      classes: node.classes || [],
      styles: node.styles || [],
    })),
    edges: graph.edges.map((edge) => ({
      id: 'edge:' + edge.id,
      edge_id: edge.id,
      source: edge.source,
      target: edge.target,
      points: [],
      styles: edge.styles || [],
    })),
    labels: [],
    bounds: { x: 0, y: 0, width: Math.max(400, graph.nodes.length * 120), height: 260 },
  });
}
export function render_graph_parts_with_layout_mode_json(graphJson) {
  return render_graph_parts_json(graphJson);
}
export function apply_graph_patch_result_json(graphJson, patchJson) {
  const graph = JSON.parse(graphJson);
  const patch = JSON.parse(patchJson);
  if (patch.op === 'add_node') {
    graph.nodes.push({ ...patch.node, classes: patch.node.classes || [], styles: patch.node.styles || [] });
  } else if (patch.op === 'add_edge') {
    graph.edges.push({ ...patch.edge, classes: patch.edge.classes || [], styles: patch.edge.styles || [] });
  } else if (patch.op === 'set_node_label') {
    graph.nodes.find((node) => node.id === patch.id).label = patch.label;
  } else if (patch.op === 'set_node_color') {
    upsertStyle(graph.nodes.find((node) => node.id === patch.id).styles, 'fill', patch.color);
  } else if (patch.op === 'move_node') {
    graph.nodes.find((node) => node.id === patch.id).position = {
      x: patch.x,
      y: patch.y,
      locked: patch.locked,
    };
  }
  return JSON.stringify({ graph, affected_ids: [] });
}

function parseToGraph(input) {
  const nodes = new Map();
  const edges = [];
  for (const rawLine of input.split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('%%') || line.startsWith('flowchart')) continue;

    const style = line.match(/^style\s+([A-Za-z0-9_]+)\s+(.+)$/);
    if (style) {
      ensureNode(nodes, style[1]);
      nodes.get(style[1]).styles = style[2].split(',').map((item) => item.trim()).filter(Boolean);
      continue;
    }

    const edge = line.match(/^([A-Za-z0-9_]+)(?:\[[^\]]*\]|\{[^}]*\}|\([^)]*\))?\s+(?:(\w+)@)?-->(?:\|[^|]*\|)?\s*([A-Za-z0-9_]+)/);
    if (edge) {
      ensureNode(nodes, edge[1]);
      ensureNode(nodes, edge[3]);
      edges.push({
        id: edge[2] || 'edge_' + edge[1] + '_' + edge[3] + '_' + edges.length,
        source: edge[1],
        target: edge[3],
        label: '',
        edge_type: '-->',
        stroke: 'normal',
        classes: [],
        styles: [],
      });
      parseNodeDeclaration(nodes, line);
      continue;
    }

    parseNodeDeclaration(nodes, line);
  }

  return {
    diagram_type: 'flowchart',
    direction: 'TB',
    nodes: [...nodes.values()],
    edges,
    classes: [],
    subgraphs: [],
  };
}

function parseNodeDeclaration(nodes, line) {
  const node = line.match(/^([A-Za-z0-9_]+)(?:\["([^"]*)"\]|\[([^\]]*)\]|\{([^}]*)\}|\(([^)]*)\))/);
  if (!node) return;
  ensureNode(nodes, node[1]);
  nodes.get(node[1]).label = node[2] || node[3] || node[4] || node[5] || node[1];
}

function ensureNode(nodes, id) {
  if (!nodes.has(id)) {
    nodes.set(id, {
      id,
      label: id,
      shape: 'square',
      classes: [],
      styles: [],
      position: null,
    });
  }
}

function renderSvg(graph) {
  const nodes = graph.nodes.map((node, index) => {
    const bounds = nodeBounds(index);
    const fill = styleValue(node.styles, 'fill') || '#ffffff';
    return '<g id="node-' + node.id + '" transform="translate(' + bounds.x + ' ' + bounds.y + ')">' +
      '<rect width="' + bounds.width + '" height="' + bounds.height + '" fill="' + fill + '"></rect>' +
      '<text>' + escapeHtml(node.label) + '</text></g>';
  }).join('');
  const edges = graph.edges.map((edge) =>
    '<g id="edge-' + edge.id + '"><path d="M 0 0 L 100 100"></path></g>'
  ).join('');
  return '<svg viewBox="0 0 1200 300">' + edges + nodes + '</svg>';
}

function nodeBounds(index) {
  return { x: 30 + index * 120, y: 80, width: 80, height: 40 };
}

function styleValue(styles, property) {
  const style = styles?.find((item) => item.startsWith(property + ':'));
  return style ? style.slice(property.length + 1) : null;
}

function upsertStyle(styles, property, value) {
  const prefix = property + ':';
  const existing = styles.findIndex((item) => item.startsWith(prefix));
  if (existing >= 0) {
    styles[existing] = prefix + value;
  } else {
    styles.push(prefix + value);
  }
}

function escapeLabel(value) {
  return String(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function escapeHtml(value) {
  return String(value).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

window.__fakeParseToGraph = parseToGraph;
`;
}

async function runBrowserAssertions(page) {
  await page.goto('/__editor_test__.html');

  const dragResult = await page.evaluate(async () => {
    const { installNodeDrag } = await import('/playground/editor-interactions.mjs');
    const preview = document.getElementById('preview');
    const root = document.getElementById('diagram-root');
    const node = document.getElementById('node-N400');
    const commits = [];

    installNodeDrag(preview, {
      getScale: () => 2,
      onCommit: (move) => commits.push({
        id: move.id,
        dx: move.dx,
        dy: move.dy,
      }),
    });

    node.dispatchEvent(new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      clientX: 20,
      clientY: 30,
    }));
    document.dispatchEvent(new MouseEvent('mousemove', {
      bubbles: true,
      cancelable: true,
      clientX: 50,
      clientY: 70,
    }));
    document.dispatchEvent(new MouseEvent('mouseup', {
      bubbles: true,
      cancelable: true,
      clientX: 50,
      clientY: 70,
    }));

    return {
      rootStable: preview.firstElementChild === root,
      transform: node.style.transform,
      commits,
    };
  });

  assert.deepEqual(dragResult, {
    rootStable: true,
    transform: 'translate(15px, 20px)',
    commits: [{ id: 'N400', dx: 15, dy: 20 }],
  });

  const inspectorResult = await page.evaluate(async () => {
    const {
      applyNodeVisualEdit,
      installNodeSelection,
      markSelectedNode,
    } = await import('/playground/editor-interactions.mjs');

    const preview = document.getElementById('preview');
    const node = document.getElementById('node-N400');
    const labelInput = document.getElementById('inspector-label');
    const colorInput = document.getElementById('inspector-color');
    const editor = document.getElementById('editor');
    const graph = {
      nodes: [{ id: 'N400', label: 'N400', styles: [] }],
    };
    const renderParts = {
      nodes: [{ id: 'node:N400', node_id: 'N400', label: 'N400', styles: [] }],
    };
    let selectedNodeId = null;

    function selectedGraphNode() {
      return graph.nodes.find((candidate) => candidate.id === selectedNodeId);
    }

    function selectedRenderPart() {
      return renderParts.nodes.find((candidate) => candidate.node_id === selectedNodeId);
    }

    function upsertStyle(styles, property, value) {
      const prefix = `${property}:`;
      const existing = styles.findIndex((style) => style.startsWith(prefix));
      if (existing >= 0) {
        styles[existing] = `${property}:${value}`;
      } else {
        styles.push(`${property}:${value}`);
      }
    }

    function updateMermaidText() {
      const graphNode = selectedGraphNode();
      editor.value = [
        'flowchart TB',
        `  ${graphNode.id}["${graphNode.label}"]`,
        `  style ${graphNode.id} ${graphNode.styles.join(',')}`,
      ].join('\n');
    }

    installNodeSelection(preview, ({ id }) => {
      selectedNodeId = id;
      markSelectedNode(preview, id);
      const graphNode = selectedGraphNode();
      labelInput.value = graphNode.label;
    });

    labelInput.addEventListener('input', () => {
      const graphNode = selectedGraphNode();
      const part = selectedRenderPart();
      graphNode.label = labelInput.value;
      part.label = labelInput.value;
      applyNodeVisualEdit(preview, graphNode.id, { label: labelInput.value });
      updateMermaidText();
    });

    colorInput.addEventListener('input', () => {
      const graphNode = selectedGraphNode();
      const part = selectedRenderPart();
      upsertStyle(graphNode.styles, 'fill', colorInput.value);
      upsertStyle(part.styles, 'fill', colorInput.value);
      applyNodeVisualEdit(preview, graphNode.id, { color: colorInput.value });
      updateMermaidText();
    });

    node.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    labelInput.value = 'Edited Node';
    labelInput.dispatchEvent(new Event('input', { bubbles: true }));
    colorInput.value = '#ff6600';
    colorInput.dispatchEvent(new Event('input', { bubbles: true }));

    return {
      graphJson: JSON.stringify(graph),
      renderPart: selectedRenderPart(),
      selected: node.classList.contains('is-selected'),
      text: node.querySelector('text').textContent,
      fill: node.querySelector('rect').getAttribute('fill'),
      editorText: editor.value,
    };
  });

  assert.deepEqual(JSON.parse(inspectorResult.graphJson), {
    nodes: [{ id: 'N400', label: 'Edited Node', styles: ['fill:#ff6600'] }],
  });
  assert.deepEqual(inspectorResult.renderPart, {
    id: 'node:N400',
    node_id: 'N400',
    label: 'Edited Node',
    styles: ['fill:#ff6600'],
  });
  assert.equal(inspectorResult.selected, true);
  assert.equal(inspectorResult.text, 'Edited Node');
  assert.equal(inspectorResult.fill, '#ff6600');
  assert.match(inspectorResult.editorText, /N400\["Edited Node"\]/);
  assert.match(inspectorResult.editorText, /style N400 fill:#ff6600/);

  const viewportResult = await page.evaluate(async () => {
    const {
      applyViewportTransform,
      createViewportState,
      fitViewportToSvg,
      panViewportBy,
      resetViewport,
      zoomViewportBy,
    } = await import('/playground/editor-interactions.mjs');

    const preview = document.getElementById('preview');
    const zoomLabel = document.getElementById('zoom-reset');
    const state = createViewportState();

    zoomViewportBy(state, 2, { x: 100, y: 80 });
    panViewportBy(state, 12, -8);
    applyViewportTransform(preview, state, zoomLabel);
    const zoomed = {
      transform: preview.style.transform,
      label: zoomLabel.textContent,
    };

    fitViewportToSvg(state, { width: 1000, height: 600 }, { width: 400, height: 300 });
    applyViewportTransform(preview, state, zoomLabel);
    const fitted = {
      scale: state.scale,
      transform: preview.style.transform,
      label: zoomLabel.textContent,
    };

    resetViewport(state);
    applyViewportTransform(preview, state, zoomLabel);
    const reset = {
      transform: preview.style.transform,
      label: zoomLabel.textContent,
    };

    return { zoomed, fitted, reset };
  });

  assert.deepEqual(viewportResult, {
    zoomed: {
      transform: 'translate(-88px, -88px) scale(2)',
      label: '200%',
    },
    fitted: {
      scale: 1.8,
      transform: 'translate(140px, 30px) scale(1.8)',
      label: '180%',
    },
    reset: {
      transform: 'translate(0px, 0px) scale(1)',
      label: '100%',
    },
  });

  const lodResult = await page.evaluate(async () => {
    const { applyViewportLevelOfDetail } = await import('/playground/editor-interactions.mjs');
    const preview = document.getElementById('preview');
    const renderParts = window.__largeRenderParts;
    const full = applyViewportLevelOfDetail(
      preview,
      { scale: 1, x: 0, y: 0 },
      renderParts,
      { width: 1200, height: 800 },
      { largeGraphThreshold: 1 },
    );
    const low = applyViewportLevelOfDetail(
      preview,
      { scale: 0.2, x: 0, y: 0 },
      renderParts,
      { width: 240, height: 180 },
      { largeGraphThreshold: 1, overscanPx: 0 },
    );

    return {
      full,
      low,
      hiddenEdgeGroups: preview.querySelectorAll(
        'g[id^="edge-"]:not([id^="edge-label-"]).selkie-lod-hidden',
      ).length,
      hiddenNodeLabels: preview.querySelectorAll('[id^="node-"] text.selkie-lod-hidden').length,
    };
  });

  assert.equal(lodResult.full.mode, 'full');
  assert.equal(lodResult.full.visibleDetailCount, 2800);
  assert.equal(lodResult.low.mode, 'lod');
  assert.ok(
    lodResult.low.renderPartUpdateCount < lodResult.full.renderPartUpdateCount,
    `expected low zoom detail count to be lower than full detail: ${JSON.stringify(lodResult)}`,
  );
  assert.ok(lodResult.low.hiddenNodeLabels > 0);
  assert.ok(lodResult.low.hiddenEdges > 0);
  assert.equal(lodResult.hiddenEdgeGroups, lodResult.low.hiddenEdges);
  assert.equal(lodResult.hiddenNodeLabels, lodResult.low.hiddenNodeLabels);
}

async function runPlaygroundCreationAssertions(page) {
  await page.goto('/playground/index.html');
  await page.waitForFunction(() =>
    document.getElementById('loading-overlay')?.classList.contains('hidden')
  );
  await page.waitForSelector('#node-A');

  await dispatchNodeClick(page, 'A');
  await page.click('#create-node');
  await page.waitForSelector('#node-Node5');

  const afterNodeCreate = await page.evaluate(() => {
    const graph = window.__fakeParseToGraph(document.getElementById('editor').value);
    return {
      hasNode: graph.nodes.some((node) => node.id === 'Node5'),
      text: document.getElementById('editor').value,
    };
  });
  assert.equal(afterNodeCreate.hasNode, true);
  assert.match(afterNodeCreate.text, /Node5\["Node5"\]/);

  await dispatchNodeClick(page, 'A');
  await page.click('#connect-edge');
  await dispatchNodeClick(page, 'Node5');

  const afterEdgeCreate = await page.evaluate(() => {
    const graph = window.__fakeParseToGraph(document.getElementById('editor').value);
    return {
      hasNode: graph.nodes.some((node) => node.id === 'Node5'),
      hasEdge: graph.edges.some((edge) => edge.source === 'A' && edge.target === 'Node5'),
      text: document.getElementById('editor').value,
    };
  });

  assert.equal(afterEdgeCreate.hasNode, true);
  assert.equal(afterEdgeCreate.hasEdge, true);
  assert.match(afterEdgeCreate.text, /A edge_A_Node5@--> Node5/);
}

async function dispatchNodeClick(page, nodeId) {
  await page.evaluate((id) => {
    const node = document.getElementById(`node-${id}`);
    node.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, nodeId);
}

const server = await serveRepo();
const browser = await chromium.launch();

try {
  const page = await browser.newPage({ baseURL: server.baseUrl });
  await runBrowserAssertions(page);
  await runPlaygroundCreationAssertions(page);
  console.log('PASS browser node drag updates the existing SVG in place');
  console.log('PASS browser node inspector edits graph data SVG and Mermaid text');
  console.log('PASS playground creates a node and edge then reparses exported text');
  console.log('PASS browser viewport pan zoom fit and reset update one transform');
  console.log('PASS browser low-zoom LOD lowers visible label and edge detail');
} finally {
  await browser.close();
  await server.close();
}
