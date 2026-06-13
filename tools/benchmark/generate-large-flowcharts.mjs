#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const outputDir = join(__dirname, 'fixtures');

const CASES = [
  [100, 200],
  [400, 600],
  [800, 1000],
  [1200, 1600],
];

function generateFlowchart(nodeCount, edgeCount) {
  const lines = ['flowchart TB'];
  const layerSize = Math.max(10, Math.round(Math.sqrt(nodeCount)));
  const layers = Math.ceil(nodeCount / layerSize);

  for (let i = 0; i < nodeCount; i += 1) {
    lines.push(`  N${i}[N${i}]`);
  }

  let edgesAdded = 0;
  for (let layer = 0; layer < layers - 1 && edgesAdded < edgeCount; layer += 1) {
    const layerStart = layer * layerSize;
    const layerEnd = Math.min(layerStart + layerSize, nodeCount);
    const nextLayerStart = (layer + 1) * layerSize;
    const nextLayerEnd = Math.min(nextLayerStart + layerSize, nodeCount);
    const nextLayerLength = nextLayerEnd - nextLayerStart;

    for (let source = layerStart; source < layerEnd && edgesAdded < edgeCount; source += 1) {
      for (let hop = 0; hop < 2 && edgesAdded < edgeCount; hop += 1) {
        const offset = source - layerStart;
        const target = nextLayerStart + ((offset + hop * 7) % nextLayerLength);
        lines.push(`  N${source} --> N${target}`);
        edgesAdded += 1;
      }
    }
  }

  let source = 0;
  while (edgesAdded < edgeCount) {
    const target = Math.min(nodeCount - 1, source + layerSize + 1);
    lines.push(`  N${source} --> N${target}`);
    edgesAdded += 1;
    source = (source + 13) % Math.max(1, nodeCount - layerSize);
  }

  return `${lines.join('\n')}\n`;
}

mkdirSync(outputDir, { recursive: true });

for (const [nodes, edges] of CASES) {
  const filename = join(outputDir, `flowchart-${nodes}-nodes-${edges}-edges.mmd`);
  writeFileSync(filename, generateFlowchart(nodes, edges));
  console.log(filename);
}
