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
  const { nodes, edges } = largeFlowchartSvgParts();
  return `<!DOCTYPE html>
  <html>
    <body>
      <div id="preview-container">
        <div id="preview">
          <svg id="diagram-root" viewBox="0 0 4000 3000">
            ${edges}
            ${nodes}
          </svg>
        </div>
      </div>
      <button id="zoom-reset"></button>
    </body>
  </html>`;
}

function largeFlowchartSvgParts() {
  const source = readFileSync(largeFixturePath, 'utf8');
  const nodeIds = [...source.matchAll(/^\s*(N\d+)\[/gm)].map((match) => match[1]);
  const edgeIds = [...source.matchAll(/^\s*N\d+\s*-->\s*N\d+/gm)].map(
    (_, index) => `edge-${index}`,
  );

  assert.equal(nodeIds.length, 800, 'large fixture should contain 800 nodes');
  assert.equal(edgeIds.length, 1000, 'large fixture should contain 1000 edges');

  return {
    nodes: nodeIds
      .map((id, index) => {
        const x = (index % 40) * 95;
        const y = Math.floor(index / 40) * 95;
        return `<g id="node-${id}" transform="translate(${x} ${y})"><rect width="80" height="40"></rect></g>`;
      })
      .join('\n'),
    edges: edgeIds
      .map((id) => `<g id="${id}"><path d="M 0 0 L 100 100"></path></g>`)
      .join('\n'),
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
}

const server = await serveRepo();
const browser = await chromium.launch();

try {
  const page = await browser.newPage({ baseURL: server.baseUrl });
  await runBrowserAssertions(page);
  console.log('PASS browser node drag updates the existing SVG in place');
  console.log('PASS browser viewport pan zoom fit and reset update one transform');
} finally {
  await browser.close();
  await server.close();
}
