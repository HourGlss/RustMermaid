#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS,
  summarizeFrameTimes,
  validateBrowserPerformanceReport,
} from './browser-performance.mjs';

assert.deepEqual(summarizeFrameTimes([4, 1, 9, 16]), {
  count: 4,
  avg_ms: 7.5,
  p95_ms: 16,
  max_ms: 16,
});

const passingReport = {
  fixture: {
    node_count: 800,
    edge_count: 1000,
  },
  initial_load: {
    interactive: true,
    load_ms: 120,
    render_ms: 80,
    dom_update_ms: 4,
    memory_estimate_bytes: 1_200_000,
  },
  drag_latency: {
    frames: summarizeFrameTimes(Array.from({ length: 60 }, (_, index) => 2 + (index % 5))),
  },
};

assert.deepEqual(
  validateBrowserPerformanceReport(passingReport, DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS),
  [],
);

const failingReport = structuredClone(passingReport);
failingReport.fixture.node_count = 799;
failingReport.initial_load.interactive = false;
failingReport.drag_latency.frames.p95_ms =
  DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS.max_drag_p95_ms + 1;

const failures = validateBrowserPerformanceReport(
  failingReport,
  DEFAULT_BROWSER_PERFORMANCE_THRESHOLDS,
);
assert.match(failures.join('\n'), /expected 800 nodes/);
assert.match(failures.join('\n'), /did not become interactive/);
assert.match(failures.join('\n'), /drag p95/);

console.log('browser-performance tests passed.');
