#!/usr/bin/env node

import { spawnSync } from 'child_process';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'fs';
import { dirname, join, relative, resolve, sep } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const DEFAULT_REPORT_DIR = join(__dirname, 'reports', 'phase6');
const TRACE_FILTER = 'selkie=trace';
const MAX_BUFFER = 256 * 1024 * 1024;
const EDITABLE_FIXTURE = join(
  __dirname,
  'fixtures',
  'flowchart-800-nodes-1000-edges.mmd',
);

const EDITABLE_SPANS = [
  'phase6.editable.parse_to_graph',
  'phase6.editable.render_graph_parts',
  'phase6.editable.move_node',
  'phase6.editable.create_node',
  'phase6.editable.create_edge',
  'phase6.editable.export',
  'phase6.editable.re_import',
];

const BUDGET_METRICS = [
  {
    metric: 'initial_load',
    source: 'workflow:cli_render',
    description: 'End-to-end CLI render wall time across docs/sources and generated fixtures.',
  },
  {
    metric: 'render_graph_parts',
    source: 'span:phase6.editable.render_graph_parts',
    description: 'Editable graph-to-render-parts workflow on the 800-node / 1000-edge fixture.',
  },
  {
    metric: 'move_node',
    source: 'span:phase6.editable.move_node',
    description: 'Patch-based node move on the 800-node / 1000-edge fixture.',
  },
  {
    metric: 'create_node',
    source: 'span:phase6.editable.create_node',
    description: 'Patch-based node creation on the 800-node / 1000-edge fixture.',
  },
  {
    metric: 'create_edge',
    source: 'span:phase6.editable.create_edge',
    description: 'Patch-based edge creation on the 800-node / 1000-edge fixture.',
  },
  {
    metric: 'export',
    source: 'span:phase6.editable.export',
    description: 'Editable graph export to Mermaid text.',
  },
  {
    metric: 're_import',
    source: 'span:phase6.editable.re_import',
    description: 'Re-import of exported Mermaid text into editable graph JSON.',
  },
];

export function parseDurationMs(value) {
  if (typeof value === 'number') {
    return value;
  }
  if (typeof value !== 'string') {
    throw new Error(`Unsupported duration value: ${value}`);
  }

  const normalized = value.trim().replace('\u00b5', 'u');
  const match = normalized.match(/^([0-9]+(?:\.[0-9]+)?)(ns|us|ms|s)$/);
  if (!match) {
    throw new Error(`Unsupported duration format: ${value}`);
  }

  const amount = Number(match[1]);
  const unit = match[2];
  if (unit === 'ns') {
    return amount / 1_000_000;
  }
  if (unit === 'us') {
    return amount / 1_000;
  }
  if (unit === 'ms') {
    return amount;
  }
  return amount * 1_000;
}

export function parseTraceText(text, metadata = {}) {
  return text
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (error) {
        throw new Error(`Invalid trace JSON on line ${index + 1}: ${error.message}`);
      }
    })
    .filter((event) => event.fields?.message === 'close' && event.fields?.['time.busy'])
    .map((event) => {
      const span = event.span?.name ?? event.spans?.at(-1)?.name;
      if (!span) {
        throw new Error(`Trace close event is missing a span name: ${JSON.stringify(event)}`);
      }
      return {
        span,
        duration_ms: parseDurationMs(event.fields['time.busy']),
        fields: event.span ?? {},
        ...metadata,
      };
    });
}

export function durationStats(values) {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((a, b) => a - b);
  const total = values.reduce((sum, value) => sum + value, 0);
  return {
    count: values.length,
    total_ms: roundMs(total),
    avg_ms: roundMs(total / values.length),
    p50_ms: roundMs(percentile(sorted, 0.5)),
    p95_ms: roundMs(percentile(sorted, 0.95)),
    max_ms: roundMs(sorted.at(-1)),
  };
}

export function summarizeEvents(events, cliTimings = [], corpus = {}) {
  const bySpan = new Map();
  for (const event of events) {
    if (!bySpan.has(event.span)) {
      bySpan.set(event.span, []);
    }
    bySpan.get(event.span).push(event.duration_ms);
  }

  const totalSpanMs = events.reduce((sum, event) => sum + event.duration_ms, 0);
  const spans = [...bySpan.entries()]
    .map(([span, values]) => {
      const stats = durationStats(values);
      return {
        span,
        ...stats,
        percent_total: totalSpanMs > 0 ? roundPercent((stats.total_ms / totalSpanMs) * 100) : 0,
      };
    })
    .sort((a, b) => b.total_ms - a.total_ms || a.span.localeCompare(b.span));

  const metricValues = new Map();
  metricValues.set(
    'initial_load',
    cliTimings.map((timing) => timing.elapsed_ms),
  );
  for (const item of BUDGET_METRICS.filter((metric) => metric.source.startsWith('span:'))) {
    const span = item.source.slice('span:'.length);
    metricValues.set(
      item.metric,
      events.filter((event) => event.span === span).map((event) => event.duration_ms),
    );
  }

  const metrics = [...metricValues.entries()].map(([metric, values]) => ({
    metric,
    ...(durationStats(values) ?? emptyStats()),
  }));

  return {
    generated_at: new Date().toISOString(),
    corpus,
    span_total_ms: roundMs(totalSpanMs),
    spans,
    metrics,
  };
}

export function buildOptimizationCandidates(summary) {
  return {
    generated_at: new Date().toISOString(),
    corpus: summary.corpus,
    candidates: summary.spans.slice(0, 10).map((span, index) => ({
      rank: index + 1,
      span: span.span,
      total_ms: span.total_ms,
      count: span.count,
      avg_ms: span.avg_ms,
      p95_ms: span.p95_ms,
      max_ms: span.max_ms,
      percent_total: span.percent_total,
      dominated_by: categoryForSpan(span.span),
      source: sourceForSpan(span.span),
      why: whyForSpan(span.span),
    })),
  };
}

export function buildBudgets(summary) {
  const metrics = new Map(summary.metrics.map((metric) => [metric.metric, metric]));
  return {
    generated_at: new Date().toISOString(),
    corpus: summary.corpus,
    policy: {
      target: 'Phase 7 should reduce each p95 by at least 15% where the metric is non-zero.',
      regression_threshold: 'Current p95 may not exceed the saved baseline p95 by more than 10%.',
    },
    metrics: BUDGET_METRICS.map((item) => {
      const stats = metrics.get(item.metric) ?? emptyStats();
      return {
        metric: item.metric,
        source: item.source,
        description: item.description,
        baseline_ms: stats,
        target_p95_ms: roundMs(stats.p95_ms * 0.85),
        regression_p95_limit_ms: roundMs(stats.p95_ms * 1.1),
      };
    }),
  };
}

export function validateManifest(manifest, expectedDocsSources) {
  const actualDocs = new Set(manifest.docs_sources ?? []);
  const missing = expectedDocsSources.filter((source) => !actualDocs.has(source));
  if (missing.length > 0) {
    throw new Error(`Trace manifest is missing docs/sources entries: ${missing.join(', ')}`);
  }
  if (actualDocs.size !== expectedDocsSources.length) {
    throw new Error(
      `Trace manifest docs/sources count mismatch: expected ${expectedDocsSources.length}, got ${actualDocs.size}`,
    );
  }
  return true;
}

export function validateTraceSpans(records, traceTextsByFile) {
  for (const record of records) {
    const traceText = traceTextsByFile.get(record.trace);
    if (traceText === undefined) {
      throw new Error(`Trace record has no loaded trace text: ${record.trace}`);
    }
    const spans = new Set(parseTraceText(traceText).map((event) => event.span));
    const expected = expectedSpansForRecord(record);
    const missing = expected.filter((span) => !spans.has(span));
    if (missing.length > 0) {
      throw new Error(`${record.trace} missing expected spans: ${missing.join(', ')}`);
    }
  }
  return true;
}

export function checkBudgetRegressions(budgets, currentSummary) {
  const currentMetrics = new Map(currentSummary.metrics.map((metric) => [metric.metric, metric]));
  const failures = [];
  for (const budget of budgets.metrics) {
    const current = currentMetrics.get(budget.metric);
    if (!current) {
      failures.push(`${budget.metric}: missing from current summary`);
      continue;
    }
    if (current.p95_ms > budget.regression_p95_limit_ms) {
      failures.push(
        `${budget.metric}: p95 ${current.p95_ms}ms exceeds limit ${budget.regression_p95_limit_ms}ms`,
      );
    }
  }
  return failures;
}

export async function runPhase6(options = {}) {
  const reportDir = resolve(options.reportDir ?? DEFAULT_REPORT_DIR);
  const traceDir = join(reportDir, 'traces');
  const renderDir = join(reportDir, 'renders');
  const editableDir = join(reportDir, 'editable');
  mkdirSync(join(traceDir, 'cli'), { recursive: true });
  mkdirSync(join(traceDir, 'editable'), { recursive: true });
  mkdirSync(renderDir, { recursive: true });
  mkdirSync(editableDir, { recursive: true });

  const docsSources = walkFiles(join(REPO_ROOT, 'docs', 'sources'), '.mmd').sort();
  const fixtureSources = fixtureFiles();
  const docsSourcesRel = docsSources.map(relativeRepoPath);
  const fixtureSourcesRel = fixtureSources.map(relativeRepoPath);
  if (!existsSync(EDITABLE_FIXTURE)) {
    throw new Error(`Missing editable workflow fixture: ${EDITABLE_FIXTURE}`);
  }

  runCommand('cargo', [
    'build',
    '--features',
    'all-formats',
    '--bin',
    'selkie',
    '--bin',
    'trace-editable-workflow',
  ]);

  const selkieBin = join(REPO_ROOT, 'target', 'debug', binName('selkie'));
  const editableBin = join(REPO_ROOT, 'target', 'debug', binName('trace-editable-workflow'));
  const records = [];
  const cliTimings = [];
  const traceTextsByFile = new Map();

  for (const input of [...docsSources, ...fixtureSources]) {
    const kind = input.startsWith(join(REPO_ROOT, 'docs', 'sources'))
      ? 'docs_source'
      : 'generated_fixture';
    const safeName = safeFileName(relativeRepoPath(input));
    const tracePath = join(traceDir, 'cli', `${safeName}.jsonl`);
    const svgPath = join(renderDir, `${safeName}.svg`);
    const started = performance.now();
    const result = spawnSync(
      selkieBin,
      ['--trace', '--trace-filter', TRACE_FILTER, input, '-o', svgPath, '--quiet'],
      {
        cwd: REPO_ROOT,
        encoding: 'utf8',
        maxBuffer: MAX_BUFFER,
      },
    );
    const elapsedMs = performance.now() - started;
    if (result.status !== 0) {
      throw new Error(
        `${relativeRepoPath(input)} failed to render:\n${result.stderr}\n${result.stdout}`,
      );
    }
    const traceText = jsonLinesOnly(result.stderr);
    writeFileSync(tracePath, traceText);
    const record = {
      kind,
      input: relativeRepoPath(input),
      trace: relativeRepoPath(tracePath),
      output: relativeRepoPath(svgPath),
      elapsed_ms: roundMs(elapsedMs),
    };
    records.push(record);
    cliTimings.push(record);
    traceTextsByFile.set(record.trace, traceText);
  }

  const editableTracePath = join(traceDir, 'editable', 'workflow-800-1000.jsonl');
  const editableSummaryPath = join(editableDir, 'workflow-800-1000.json');
  const editableResult = spawnSync(
    editableBin,
    [EDITABLE_FIXTURE, '--summary-output', editableSummaryPath],
    {
      cwd: REPO_ROOT,
      encoding: 'utf8',
      env: { ...process.env, SELKIE_TRACE_FILTER: TRACE_FILTER },
      maxBuffer: MAX_BUFFER,
    },
  );
  if (editableResult.status !== 0) {
    throw new Error(
      `Editable workflow failed:\n${editableResult.stderr}\n${editableResult.stdout}`,
    );
  }
  const editableTraceText = jsonLinesOnly(editableResult.stderr);
  writeFileSync(editableTracePath, editableTraceText);
  const editableRecord = {
    kind: 'editable_workflow',
    input: relativeRepoPath(EDITABLE_FIXTURE),
    trace: relativeRepoPath(editableTracePath),
    output: relativeRepoPath(editableSummaryPath),
    elapsed_ms: null,
  };
  records.push(editableRecord);
  traceTextsByFile.set(editableRecord.trace, editableTraceText);

  const manifest = {
    generated_at: new Date().toISOString(),
    docs_sources_count: docsSourcesRel.length,
    docs_sources: docsSourcesRel,
    generated_fixtures_count: fixtureSourcesRel.length,
    generated_fixtures: fixtureSourcesRel,
    editable_fixture: relativeRepoPath(EDITABLE_FIXTURE),
    traces: records,
  };
  validateManifest(manifest, docsSourcesRel);
  validateTraceSpans(records, traceTextsByFile);

  const events = records.flatMap((record) =>
    parseTraceText(traceTextsByFile.get(record.trace), {
      trace: record.trace,
      input: record.input,
      kind: record.kind,
    }),
  );
  const corpus = {
    docs_sources_count: docsSourcesRel.length,
    generated_fixtures_count: fixtureSourcesRel.length,
    editable_workflow_count: 1,
    trace_count: records.length,
  };
  const summary = summarizeEvents(events, cliTimings, corpus);
  const candidates = buildOptimizationCandidates(summary);
  const budgets = buildBudgets(summary);
  const budgetFailures = checkBudgetRegressions(budgets, summary);
  if (budgetFailures.length > 0) {
    throw new Error(`Freshly generated budgets failed comparison:\n${budgetFailures.join('\n')}`);
  }

  writeJson(join(reportDir, 'manifest.json'), manifest);
  writeJson(join(reportDir, 'summary.json'), summary);
  writeJson(join(reportDir, 'optimization-candidates.json'), candidates);
  writeFileSync(
    join(reportDir, 'optimization-candidates.md'),
    renderCandidatesMarkdown(candidates),
  );
  writeJson(join(reportDir, 'performance-budgets.json'), budgets);
  writeFileSync(join(reportDir, 'summary.md'), renderSummaryMarkdown(summary, budgets));

  return {
    reportDir,
    manifest,
    summary,
    candidates,
    budgets,
  };
}

function expectedSpansForRecord(record) {
  if (record.kind === 'editable_workflow') {
    return [
      ...EDITABLE_SPANS,
      'selkie.editable.parse_to_graph',
      'selkie.editable.render_graph_parts',
      'selkie.editable.apply_graph_patch',
      'selkie.editable.graph_to_mermaid_text',
    ];
  }

  const expected = ['selkie.parse', 'selkie.render_with_config'];
  if (record.kind === 'generated_fixture') {
    expected.push(
      'selkie.render.flowchart',
      'selkie.render.flowchart.layout',
      'selkie.layout.dagre',
      'selkie.render.flowchart.svg',
    );
  }
  return expected;
}

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    cwd: REPO_ROOT,
    stdio: 'inherit',
    encoding: 'utf8',
    maxBuffer: MAX_BUFFER,
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed`);
  }
}

function fixtureFiles() {
  const fixtureDir = join(__dirname, 'fixtures');
  return readdirSync(fixtureDir)
    .filter((file) => file.endsWith('.mmd'))
    .map((file) => join(fixtureDir, file))
    .sort((a, b) => fixtureSize(a) - fixtureSize(b));
}

function fixtureSize(path) {
  const match = path.match(/flowchart-([0-9]+)-nodes/);
  return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
}

function walkFiles(dir, extension) {
  const files = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      files.push(...walkFiles(path, extension));
    } else if (path.endsWith(extension)) {
      files.push(path);
    }
  }
  return files;
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function jsonLinesOnly(text) {
  const lines = text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('{'));
  if (lines.length === 0) {
    throw new Error(`Expected JSON trace lines, got:\n${text}`);
  }
  return `${lines.join('\n')}\n`;
}

function safeFileName(path) {
  return path.replace(/[^a-zA-Z0-9._-]+/g, '_');
}

function relativeRepoPath(path) {
  return relative(REPO_ROOT, path).split(sep).join('/');
}

function binName(name) {
  return process.platform === 'win32' ? `${name}.exe` : name;
}

function percentile(sortedValues, quantile) {
  const index = Math.max(0, Math.ceil(sortedValues.length * quantile) - 1);
  return sortedValues[index];
}

function roundMs(value) {
  return Number((value ?? 0).toFixed(3));
}

function roundPercent(value) {
  return Number(value.toFixed(2));
}

function emptyStats() {
  return {
    count: 0,
    total_ms: 0,
    avg_ms: 0,
    p50_ms: 0,
    p95_ms: 0,
    max_ms: 0,
  };
}

function categoryForSpan(span) {
  if (span.includes('layout') || span.includes('dagre')) {
    return 'layout';
  }
  if (span.includes('graph_to_mermaid') || span.includes('export')) {
    return 'serialization';
  }
  if (span.includes('render_graph_parts')) {
    return 'DOM/update';
  }
  if (span.includes('render') || span.includes('svg')) {
    return 'render';
  }
  if (span.includes('parse') || span.includes('apply_graph_patch') || span.includes('create_')) {
    return 'CPU';
  }
  return 'CPU';
}

function sourceForSpan(span) {
  if (span.includes('dagre')) {
    return 'src/layout/dagre/mod.rs';
  }
  if (span.includes('layout')) {
    return 'src/layout/mod.rs';
  }
  if (span.includes('editable') || span.includes('graph_to_mermaid')) {
    return 'src/editable.rs';
  }
  if (span === 'selkie.render_with_config') {
    return 'src/render/mod.rs';
  }
  if (span.includes('flowchart') || span.includes('render')) {
    return 'src/render/flowchart.rs';
  }
  if (span.includes('parse')) {
    return 'src/lib.rs';
  }
  return 'src/lib.rs';
}

function whyForSpan(span) {
  if (span.includes('layout') || span.includes('dagre')) {
    return 'Large flowcharts spend repeated wall time here as node and edge counts grow.';
  }
  if (span.includes('render_graph_parts')) {
    return 'This is on the editor interaction path before DOM updates can be applied.';
  }
  if (span.includes('apply_graph_patch')) {
    return 'Patch cost directly affects move/create/edit latency in the live graph editor.';
  }
  if (span.includes('graph_to_mermaid') || span.includes('export')) {
    return 'Export cost affects round-trip editing and save latency.';
  }
  if (span.includes('parse')) {
    return 'Parsing is paid on initial load and after text edits or re-imports.';
  }
  return 'It ranks high by total traced time across the Phase 6 corpus.';
}

function renderCandidatesMarkdown(report) {
  const lines = [
    '# Phase 6 Optimization Candidates',
    '',
    `Docs sources traced: ${report.corpus.docs_sources_count}`,
    `Generated fixtures traced: ${report.corpus.generated_fixtures_count}`,
    '',
    '| Rank | Span | Total ms | P95 ms | Count | Dominated by | Source |',
    '| ---: | --- | ---: | ---: | ---: | --- | --- |',
  ];
  for (const item of report.candidates) {
    lines.push(
      `| ${item.rank} | \`${item.span}\` | ${item.total_ms} | ${item.p95_ms} | ${item.count} | ${item.dominated_by} | \`${item.source}\` |`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function renderSummaryMarkdown(summary, budgets) {
  const lines = [
    '# Phase 6 Trace Summary',
    '',
    `Docs sources traced: ${summary.corpus.docs_sources_count}`,
    `Generated fixtures traced: ${summary.corpus.generated_fixtures_count}`,
    `Trace files: ${summary.corpus.trace_count}`,
    '',
    '## Top Spans',
    '',
    '| Span | Total ms | P95 ms | Count | Percent |',
    '| --- | ---: | ---: | ---: | ---: |',
  ];
  for (const item of summary.spans.slice(0, 10)) {
    lines.push(
      `| \`${item.span}\` | ${item.total_ms} | ${item.p95_ms} | ${item.count} | ${item.percent_total}% |`,
    );
  }
  lines.push('', '## Phase 7 Budgets', '');
  lines.push('| Metric | Baseline p95 ms | Target p95 ms | Regression limit ms |');
  lines.push('| --- | ---: | ---: | ---: |');
  for (const item of budgets.metrics) {
    lines.push(
      `| \`${item.metric}\` | ${item.baseline_ms.p95_ms} | ${item.target_p95_ms} | ${item.regression_p95_limit_ms} |`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

async function main() {
  const args = process.argv.slice(2);
  if (args[0] === 'check-budgets') {
    const budgetsPath = args[1];
    const summaryPath = args[2];
    if (!budgetsPath || !summaryPath) {
      throw new Error(
        'usage: phase6-trace.mjs check-budgets <performance-budgets.json> <summary.json>',
      );
    }
    const failures = checkBudgetRegressions(loadJson(budgetsPath), loadJson(summaryPath));
    if (failures.length > 0) {
      throw new Error(`Budget regressions:\n${failures.join('\n')}`);
    }
    console.log('Phase 6 budgets passed.');
    return;
  }

  const reportDirIndex = args.indexOf('--report-dir');
  const reportDir =
    reportDirIndex === -1 ? DEFAULT_REPORT_DIR : resolve(args[reportDirIndex + 1]);
  const result = await runPhase6({ reportDir });
  console.log(`Phase 6 trace reports written to ${relativeRepoPath(result.reportDir)}`);
  console.log(
    `Traced ${result.manifest.docs_sources_count} docs sources and ${result.manifest.generated_fixtures_count} generated fixtures.`,
  );
}

const invokedAsScript =
  process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (invokedAsScript) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
