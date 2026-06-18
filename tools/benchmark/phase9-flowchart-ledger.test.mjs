import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  buildFlowchartIssueLedger,
  parseLedgerArgs,
  validateLedgerCounts
} from './phase9-flowchart-ledger.mjs';

test('builds a Phase 9 ledger grouped by issue category and owner task', async () => {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), 'selkie-phase9-ledger-'));
  try {
    await mkdir(path.join(tempDir, 'flowchart'));
    await writeFile(
      path.join(tempDir, 'report.json'),
      JSON.stringify({
        total: 3,
        matching: 1,
        by_type: {
          flowchart: {
            total: 2,
            matching: 1,
            avg_structural: 0.9
          }
        },
        diagrams: [
          {
            name: 'fixture_a',
            diagram_type: 'flowchart',
            status: 'error',
            json_file: 'flowchart/fixture_a_comparison.json'
          },
          {
            name: 'fixture_b',
            diagram_type: 'flowchart',
            status: 'warning',
            json_file: 'flowchart/fixture_b_comparison.json'
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
        name: 'fixture_a',
        status: 'error',
        issues: [
          { level: 'error', check: 'aspect_ratio', message: 'too tall' },
          { level: 'warning', check: 'edge_positions', message: 'route differs' }
        ]
      })
    );
    await writeFile(
      path.join(tempDir, 'flowchart', 'fixture_b_comparison.json'),
      JSON.stringify({
        name: 'fixture_b',
        status: 'warning',
        issues: [
          { level: 'info', check: 'colors', message: 'color differs' }
        ]
      })
    );

    const ledger = await buildFlowchartIssueLedger(tempDir);

    assert.deepEqual(ledger.counts, {
      errors: 1,
      warnings: 1,
      info: 1,
      visual_only: 0
    });
    assert.deepEqual(ledger.categories, {
      'error:aspect_ratio': 1,
      'info:colors': 1,
      'warning:edge_positions': 1
    });
    assert.equal(ledger.entries.length, 3);
    assert.equal(ledger.entries[0].owner_task, 'phase9-layout-parity');
    assert.equal(ledger.entries[1].owner_task, 'phase9-edge-routing-parity');
    assert.equal(ledger.entries[2].owner_task, 'phase9-style-parity');
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
});

test('validates ledger counts exactly', () => {
  const ledger = {
    counts: {
      errors: 1,
      warnings: 2,
      info: 3
    }
  };

  assert.deepEqual(
    validateLedgerCounts(ledger, { errors: 1, warnings: 2, info: 3 }),
    []
  );
  assert.deepEqual(validateLedgerCounts(ledger, { errors: 0 }), [
    'errors expected 0, got 1'
  ]);
});

test('parses ledger CLI arguments', () => {
  assert.deepEqual(
    parseLedgerArgs([
      'eval-report/selkie-eval-test',
      '--write',
      'tools/benchmark/reports/ledger.json',
      '--expect',
      'errors=9,warnings=63,info=92'
    ]),
    {
      evalDir: 'eval-report/selkie-eval-test',
      writePath: 'tools/benchmark/reports/ledger.json',
      expect: {
        errors: 9,
        warnings: 63,
        info: 92
      }
    }
  );
});
