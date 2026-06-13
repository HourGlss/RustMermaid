#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  buildBudgets,
  buildOptimizationCandidates,
  checkBudgetRegressions,
  parseDurationMs,
  parseTraceText,
  summarizeEvents,
  validateManifest,
  validateTraceSpans,
} from './phase6-trace.mjs';

function traceLine(span, duration) {
  return JSON.stringify({
    fields: {
      message: 'close',
      'time.busy': duration,
    },
    span: {
      name: span,
    },
  });
}

assert.equal(parseDurationMs('1s'), 1000);
assert.equal(parseDurationMs('2.5ms'), 2.5);
assert.equal(parseDurationMs('250us'), 0.25);
assert.equal(parseDurationMs('1000ns'), 0.001);
assert.equal(parseDurationMs('3\u00b5s'), 0.003);

const traceText = [
  traceLine('selkie.parse', '1ms'),
  traceLine('selkie.parse', '3ms'),
  traceLine('selkie.layout.dagre', '10ms'),
  traceLine('phase6.editable.render_graph_parts', '4ms'),
  traceLine('phase6.editable.move_node', '2ms'),
  traceLine('phase6.editable.create_node', '2ms'),
  traceLine('phase6.editable.create_edge', '2ms'),
  traceLine('phase6.editable.export', '5ms'),
  traceLine('phase6.editable.re_import', '6ms'),
].join('\n');
const events = parseTraceText(traceText, { input: 'fixture.mmd' });
assert.equal(events.length, 9);
assert.equal(events[0].duration_ms, 1);

const summary = summarizeEvents(
  events,
  [
    { elapsed_ms: 12 },
    { elapsed_ms: 18 },
  ],
  {
    docs_sources_count: 76,
    generated_fixtures_count: 4,
    editable_workflow_count: 1,
    trace_count: 81,
  },
);
assert.equal(summary.spans[0].span, 'selkie.layout.dagre');
assert.equal(summary.spans[0].total_ms, 10);
assert.equal(summary.metrics.find((metric) => metric.metric === 'initial_load').p95_ms, 18);

const candidates = buildOptimizationCandidates(summary);
assert.equal(candidates.candidates.length, 8);
assert.equal(candidates.candidates[0].source, 'src/layout/dagre/mod.rs');
assert.equal(candidates.corpus.docs_sources_count, 76);

const budgets = buildBudgets(summary);
assert.equal(
  budgets.metrics.find((metric) => metric.metric === 'initial_load').regression_p95_limit_ms,
  19.8,
);
assert.deepEqual(checkBudgetRegressions(budgets, summary), []);

const regressedSummary = structuredClone(summary);
regressedSummary.metrics.find((metric) => metric.metric === 'initial_load').p95_ms = 20;
assert.match(checkBudgetRegressions(budgets, regressedSummary)[0], /initial_load/);

assert.equal(
  validateManifest(
    { docs_sources: ['docs/sources/a.mmd'], docs_sources_count: 1 },
    ['docs/sources/a.mmd'],
  ),
  true,
);
assert.throws(
  () => validateManifest({ docs_sources: [] }, ['docs/sources/a.mmd']),
  /missing docs\/sources/,
);

const cliTrace = [
  traceLine('selkie.parse', '1ms'),
  traceLine('selkie.render_with_config', '2ms'),
  traceLine('selkie.render.flowchart', '3ms'),
  traceLine('selkie.render.flowchart.layout', '4ms'),
  traceLine('selkie.layout.dagre', '5ms'),
  traceLine('selkie.render.flowchart.svg', '6ms'),
].join('\n');
const editableTrace = [
  ...[
    'phase6.editable.parse_to_graph',
    'phase6.editable.render_graph_parts',
    'phase6.editable.move_node',
    'phase6.editable.create_node',
    'phase6.editable.create_edge',
    'phase6.editable.export',
    'phase6.editable.re_import',
    'selkie.editable.parse_to_graph',
    'selkie.editable.render_graph_parts',
    'selkie.editable.apply_graph_patch',
    'selkie.editable.graph_to_mermaid_text',
  ].map((span) => traceLine(span, '1ms')),
].join('\n');
assert.equal(
  validateTraceSpans(
    [
      { kind: 'generated_fixture', trace: 'fixture.jsonl' },
      { kind: 'editable_workflow', trace: 'editable.jsonl' },
    ],
    new Map([
      ['fixture.jsonl', cliTrace],
      ['editable.jsonl', editableTrace],
    ]),
  ),
  true,
);

assert.throws(
  () =>
    validateTraceSpans(
      [{ kind: 'generated_fixture', trace: 'missing-layout.jsonl' }],
      new Map([['missing-layout.jsonl', traceLine('selkie.parse', '1ms')]]),
    ),
  /missing expected spans/,
);

console.log('phase6-trace tests passed.');
