#!/usr/bin/env node

import { createReadStream, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const LARGE_FIXTURE = resolve(
  __dirname,
  'fixtures',
  'flowchart-800-nodes-1000-edges.mmd',
);
const DEFAULT_REPORT_PATH = resolve(__dirname, 'reports', 'browser-performance.json');

const CONTENT_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
};

export const DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS = {
  expected_node_count: 800,
  expected_edge_count: 1000,
  max_initial_load_ms: 5_000,
  max_drag_p95_ms: 50,
};

export function summarizeFrameTimes(values) {
  if (values.length === 0) {
    return { count: 0, avg_ms: 0, p95_ms: 0, max_ms: 0 };
  }
  const sorted = [...values].sort((a, b) => a - b);
  const total = values.reduce((sum, value) => sum + value, 0);
  return {
    count: values.length,
    avg_ms: roundMs(total / values.length),
    p95_ms: roundMs(percentile(sorted, 0.95)),
    max_ms: roundMs(sorted.at(-1)),
  };
}

export function validateBrowserPerformanceReport(
  report,
  thresholds = DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS,
) {
  const failures = [];
  if (report.fixture?.node_count !== thresholds.expected_node_count) {
    failures.push(
      `expected ${thresholds.expected_node_count} nodes, got ${report.fixture?.node_count}`,
    );
  }
  if (report.fixture?.edge_count !== thresholds.expected_edge_count) {
    failures.push(
      `expected ${thresholds.expected_edge_count} edges, got ${report.fixture?.edge_count}`,
    );
  }
  if (!report.initial_load?.interactive) {
    failures.push('browser benchmark did not become interactive');
  }
  if (report.initial_load?.load_ms > thresholds.max_initial_load_ms) {
    failures.push(
      `initial load ${report.initial_load.load_ms}ms exceeds limit ${thresholds.max_initial_load_ms}ms`,
    );
  }
  if (!Number.isFinite(report.initial_load?.memory_estimate_bytes)) {
    failures.push('memory estimate is missing or non-finite');
  }
  if (report.drag_latency?.frames?.p95_ms > thresholds.max_drag_p95_ms) {
    failures.push(
      `drag p95 ${report.drag_latency.frames.p95_ms}ms exceeds limit ${thresholds.max_drag_p95_ms}ms`,
    );
  }
  if ((report.drag_latency?.frames?.count ?? 0) < 60) {
    failures.push(`expected at least 60 drag frames, got ${report.drag_latency?.frames?.count ?? 0}`);
  }
  return failures;
}

export async function runBrowserPerformanceBenchmark(options = {}) {
  const reportPath = resolve(options.reportPath ?? DEFAULT_REPORT_PATH);
  const thresholds = {
    ...DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS,
    ...(options.thresholds ?? {}),
  };
  const fixture = parseLargeFixture(readFileSync(options.fixturePath ?? LARGE_FIXTURE, 'utf8'));
  const server = await serveBenchmark(fixture);
  const browser = await chromium.launch();

  try {
    const page = await browser.newPage({ baseURL: server.baseUrl });
    await page.goto('/__browser_performance__.html', { waitUntil: 'load' });
    await page.waitForFunction(() => window.__selkieBrowserBenchmark);

    const initial = await page.evaluate(() => window.__selkieBrowserBenchmark);
    const drag = await page.evaluate(async () => window.__runSelkieDragBenchmark());
    const report = {
      generated_at: new Date().toISOString(),
      report_path: relativeRepoPath(reportPath),
      fixture: {
        path: relativeRepoPath(options.fixturePath ?? LARGE_FIXTURE),
        node_count: fixture.nodes.length,
        edge_count: fixture.edges.length,
      },
      initial_load: initial.initial_load,
      level_of_detail: initial.level_of_detail,
      drag_latency: {
        frames: summarizeFrameTimes(drag.frame_times_ms),
        raw_frames_ms: drag.frame_times_ms.map(roundMs),
        commit_count: drag.commit_count,
        final_transform: drag.final_transform,
      },
    };

    const failures = validateBrowserPerformanceReport(report, thresholds);
    if (failures.length > 0) {
      throw new Error(`Browser performance acceptance failed:\n${failures.join('\n')}`);
    }

    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    return report;
  } finally {
    await browser.close();
    await server.close();
  }
}

function parseLargeFixture(source) {
  const nodes = [...source.matchAll(/^\s*(N\d+)\[/gm)].map((match, index) => ({
    id: match[1],
    label: match[1],
    bounds: nodeBounds(index),
  }));
  const edges = [...source.matchAll(/^\s*(N\d+)\s*-->\s*(N\d+)/gm)].map((match, index) => ({
    id: String(index),
    source: match[1],
    target: match[2],
  }));
  return { nodes, edges };
}

function nodeBounds(index) {
  return {
    x: (index % 40) * 95,
    y: Math.floor(index / 40) * 95,
    width: 80,
    height: 40,
  };
}

async function serveBenchmark(fixture) {
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? '/', 'http://127.0.0.1');
      if (url.pathname === '/__browser_performance__.html') {
        response.writeHead(200, { 'content-type': CONTENT_TYPES['.html'] });
        response.end(benchmarkHtml(fixture));
        return;
      }

      const relativePath = decodeURIComponent(url.pathname.replace(/^\/+/, ''));
      const filePath = resolve(REPO_ROOT, relativePath);
      if (!filePath.startsWith(REPO_ROOT)) {
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
        'content-type': CONTENT_TYPES[extname(filePath)] ?? 'application/octet-stream',
      });
      createReadStream(filePath).pipe(response);
    } catch {
      response.writeHead(404);
      response.end('Not found');
    }
  });

  await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
  return {
    baseUrl: `http://127.0.0.1:${server.address().port}`,
    close: () => new Promise((resolveClose) => server.close(resolveClose)),
  };
}

function benchmarkHtml(fixture) {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      body { margin: 0; overflow: hidden; }
      #preview-shell { width: 1200px; height: 800px; overflow: hidden; }
      #preview { transform-origin: 0 0; }
      .selkie-lod-hidden { display: none; }
    </style>
  </head>
  <body>
    <div id="preview-shell"><div id="preview"></div></div>
    <script type="module">
      import {
        applyViewportLevelOfDetail,
        installNodeDrag,
      } from '/playground/editor-interactions.mjs';

      const fixture = ${JSON.stringify(fixture)};
      const startedAt = performance.now();
      const preview = document.getElementById('preview');
      const renderParts = {
        nodes: fixture.nodes.map((node) => ({
          id: 'node:' + node.id,
          node_id: node.id,
          label: node.label,
          bounds: node.bounds,
          classes: [],
          styles: [],
        })),
        edges: fixture.edges.map((edge) => ({
          id: 'edge:' + edge.id,
          edge_id: edge.id,
          source: edge.source,
          target: edge.target,
          points: [],
          styles: [],
        })),
        labels: [],
        bounds: { x: 0, y: 0, width: 4000, height: 3000 },
      };

      const renderStart = performance.now();
      preview.appendChild(buildSvg(fixture));
      const renderMs = performance.now() - renderStart;

      const domUpdateStart = performance.now();
      const lodStats = applyViewportLevelOfDetail(
        preview,
        { scale: 0.25, x: 0, y: 0 },
        renderParts,
        { width: 1200, height: 800 },
        { largeGraphThreshold: 1, overscanPx: 0 },
      );
      const domUpdateMs = performance.now() - domUpdateStart;

      const commits = [];
      installNodeDrag(preview, {
        getScale: () => 1,
        onCommit: (move) => commits.push(move),
      });

      window.__selkieBrowserBenchmark = {
        initial_load: {
          interactive: document.querySelectorAll('[id^="node-N"]').length === 800 &&
            document.querySelectorAll('g[id^="edge-"]:not([id^="edge-label-"])').length === 1000,
          load_ms: performance.now() - startedAt,
          render_ms: renderMs,
          dom_update_ms: domUpdateMs,
          memory_estimate_bytes: estimateMemoryBytes(),
          node_element_count: document.querySelectorAll('[id^="node-N"]').length,
          edge_element_count: document.querySelectorAll('g[id^="edge-"]:not([id^="edge-label-"])').length,
        },
        level_of_detail: lodStats,
      };

      window.__runSelkieDragBenchmark = async () => {
        const frameTimes = [];
        const node = document.getElementById('node-N400');
        node.dispatchEvent(new MouseEvent('mousedown', {
          bubbles: true,
          cancelable: true,
          clientX: 100,
          clientY: 100,
        }));

        for (let frame = 1; frame <= 60; frame += 1) {
          const frameStart = performance.now();
          document.dispatchEvent(new MouseEvent('mousemove', {
            bubbles: true,
            cancelable: true,
            clientX: 100 + frame,
            clientY: 100 + frame,
          }));
          await new Promise((resolve) => requestAnimationFrame(resolve));
          frameTimes.push(performance.now() - frameStart);
        }

        document.dispatchEvent(new MouseEvent('mouseup', {
          bubbles: true,
          cancelable: true,
          clientX: 160,
          clientY: 160,
        }));

        return {
          frame_times_ms: frameTimes,
          commit_count: commits.length,
          final_transform: node.style.transform,
        };
      };

      function buildSvg(data) {
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('id', 'diagram-root');
        svg.setAttribute('viewBox', '0 0 4000 3000');

        const edgeLayer = document.createElementNS(svg.namespaceURI, 'g');
        edgeLayer.setAttribute('class', 'edges');
        for (const edge of data.edges) {
          const group = document.createElementNS(svg.namespaceURI, 'g');
          group.setAttribute('id', 'edge-' + edge.id);
          const path = document.createElementNS(svg.namespaceURI, 'path');
          path.setAttribute('d', 'M 0 0 L 100 100');
          group.appendChild(path);
          edgeLayer.appendChild(group);
        }
        svg.appendChild(edgeLayer);

        const nodesLayer = document.createElementNS(svg.namespaceURI, 'g');
        nodesLayer.setAttribute('class', 'nodes');
        for (const node of data.nodes) {
          const group = document.createElementNS(svg.namespaceURI, 'g');
          group.setAttribute('id', 'node-' + node.id);
          group.setAttribute('transform', 'translate(' + node.bounds.x + ' ' + node.bounds.y + ')');
          const rect = document.createElementNS(svg.namespaceURI, 'rect');
          rect.setAttribute('width', node.bounds.width);
          rect.setAttribute('height', node.bounds.height);
          const text = document.createElementNS(svg.namespaceURI, 'text');
          text.textContent = node.label;
          group.append(rect, text);
          nodesLayer.appendChild(group);
        }
        svg.appendChild(nodesLayer);
        return svg;
      }

      function estimateMemoryBytes() {
        if (performance.memory?.usedJSHeapSize) {
          return performance.memory.usedJSHeapSize;
        }
        return document.documentElement.outerHTML.length * 2;
      }
    </script>
  </body>
</html>`;
}

function percentile(sortedValues, quantile) {
  const index = Math.max(0, Math.ceil(sortedValues.length * quantile) - 1);
  return sortedValues[index];
}

function roundMs(value) {
  return Number((value ?? 0).toFixed(3));
}

function relativeRepoPath(path) {
  return resolve(path).startsWith(REPO_ROOT)
    ? resolve(path).slice(REPO_ROOT.length + 1).replaceAll('\\', '/')
    : path;
}

function parseCliArgs(args) {
  const options = { thresholds: {} };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === '--report') {
      options.reportPath = args[++i];
    } else if (arg === '--fixture') {
      options.fixturePath = args[++i];
    } else if (arg === '--max-drag-p95-ms') {
      options.thresholds.max_drag_p95_ms = Number(args[++i]);
    } else if (arg === '--max-initial-load-ms') {
      options.thresholds.max_initial_load_ms = Number(args[++i]);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return options;
}

const invokedAsScript =
  process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (invokedAsScript) {
  runBrowserPerformanceBenchmark(parseCliArgs(process.argv.slice(2)))
    .then((report) => {
      console.log(
        `Browser performance report written: ${report.report_path} ` +
          `(load ${report.initial_load.load_ms}ms, drag p95 ${report.drag_latency.frames.p95_ms}ms)`,
      );
    })
    .catch((error) => {
      console.error(error.message);
      process.exit(1);
    });
}
