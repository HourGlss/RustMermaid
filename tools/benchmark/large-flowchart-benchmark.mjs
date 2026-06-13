#!/usr/bin/env node

import { spawnSync } from 'child_process';
import { existsSync, mkdirSync, readdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixtureDir = join(__dirname, 'fixtures');
const outputDir = join(__dirname, 'reports');
const outputFile = join(outputDir, 'large-flowchart-benchmark.json');

if (!existsSync(fixtureDir)) {
  console.error(`Missing fixture directory: ${fixtureDir}`);
  console.error('Run: node tools/benchmark/generate-large-flowcharts.mjs');
  process.exit(1);
}

mkdirSync(outputDir, { recursive: true });

const fixtures = readdirSync(fixtureDir)
  .filter((file) => file.endsWith('.mmd'))
  .sort()
  .map((file) => join(fixtureDir, file));

const result = spawnSync(
  'cargo',
  ['run', '--release', '--bin', 'flowchart_benchmark', '--', '--output', outputFile, ...fixtures],
  {
    cwd: join(__dirname, '..', '..'),
    stdio: 'inherit',
  },
);

process.exit(result.status ?? 1);
