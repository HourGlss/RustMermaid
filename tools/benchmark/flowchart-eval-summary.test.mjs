import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  checkSummaryAgainstTarget,
  parseSummaryArgs,
  summarizeFlowchartEval
} from './flowchart-eval-summary.mjs';

test('summarizes the Phase 8 flowchart eval baseline totals', async () => {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), 'selkie-flowchart-eval-'));
  try {
    await mkdir(path.join(tempDir, 'flowchart'));
    await writeFile(
      path.join(tempDir, 'report.json'),
      JSON.stringify({
        total: 34,
        matching: 1,
        parity_percent: 2.9411764705882355,
        by_type: {
          flowchart: {
            total: 34,
            matching: 1,
            parity_percent: 2.9411764705882355,
            avg_structural: 0.8902145698730313
          }
        },
        issue_counts: {
          errors: 29,
          warnings: 66,
          info: 95,
          visual_only: 0
        },
        diagrams: [
          {
            name: 'fixture_a',
            diagram_type: 'flowchart',
            json_file: 'flowchart/fixture_a_comparison.json'
          },
          {
            name: 'sequence_fixture',
            diagram_type: 'sequence',
            json_file: 'sequence/sequence_fixture_comparison.json'
          }
        ]
      })
    );
    await writeFile(
      path.join(tempDir, 'flowchart', 'fixture_a_comparison.json'),
      JSON.stringify({
        issues: [
          { level: 'error', check: 'labels_missing' },
          { level: 'warning', check: 'edge_positions' },
          { level: 'warning', check: 'edge_positions' },
          { level: 'info', check: 'colors' }
        ]
      })
    );

    const summary = await summarizeFlowchartEval(tempDir);

    assert.equal(summary.total, 34);
    assert.equal(summary.matching, 1);
    assert.equal(summary.issue_counts.errors, 29);
    assert.equal(summary.issue_counts.warnings, 66);
    assert.equal(summary.issue_counts.info, 95);
    assert.equal(summary.avg_structural, 0.8902145698730313);
    assert.deepEqual(summary.issue_categories, {
      'warning:edge_positions': 2,
      'error:labels_missing': 1,
      'info:colors': 1
    });
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
});

test('parses positional eval directory without --write', () => {
  assert.deepEqual(parseSummaryArgs(['eval-report/phase8-after/selkie-eval-1234']), {
    evalDir: 'eval-report/phase8-after/selkie-eval-1234',
    writePath: null
  });
});

test('parses positional eval directory with --write', () => {
  assert.deepEqual(
    parseSummaryArgs([
      'eval-report/phase8-after/selkie-eval-1234',
      '--write',
      'tools/benchmark/baselines/flowchart-eval-baseline.json'
    ]),
    {
      evalDir: 'eval-report/phase8-after/selkie-eval-1234',
      writePath: 'tools/benchmark/baselines/flowchart-eval-baseline.json'
    }
  );
});

test('checks flowchart eval summaries against target thresholds', () => {
  const summary = {
    matching: 1,
    avg_structural: 0.9075,
    issue_counts: {
      errors: 15,
      warnings: 65,
      info: 91
    },
    issue_categories: {
      'error:labels_missing': 2,
      'error:node_count': 1
    }
  };
  const target = {
    min_matching: 1,
    min_avg_structural: 0.9,
    max_errors: 15,
    max_warnings: 65,
    max_info: 91,
    max_issue_categories: {
      'error:labels_missing': 2,
      'error:node_count': 1
    }
  };

  assert.deepEqual(checkSummaryAgainstTarget(summary, target), []);
  assert.deepEqual(checkSummaryAgainstTarget({ ...summary, matching: 0 }, target), [
    'matching expected >= 1, got 0'
  ]);
  assert.deepEqual(
    checkSummaryAgainstTarget(
      {
        ...summary,
        issue_categories: {
          ...summary.issue_categories,
          'error:labels_missing': 3
        }
      },
      target
    ),
    ['issue_categories.error:labels_missing expected <= 2, got 3']
  );
});
