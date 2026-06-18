import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '../..');

export async function buildFlowchartIssueLedger(evalDir) {
  const reportPath = path.join(evalDir, 'report.json');
  const report = JSON.parse(await readFile(reportPath, 'utf8'));
  const entries = [];
  const categories = {};
  const counts = { errors: 0, warnings: 0, info: 0, visual_only: 0 };
  const flowchartDiagrams = (report.diagrams ?? []).filter(
    (diagram) => diagram.diagram_type === 'flowchart' && diagram.json_file
  );

  for (const diagram of flowchartDiagrams) {
    const comparisonPath = path.join(evalDir, diagram.json_file);
    const comparison = JSON.parse(await readFile(comparisonPath, 'utf8'));
    for (const issue of comparison.issues ?? []) {
      const severity = issue.level ?? 'unknown';
      const check = issue.check ?? 'unknown';
      const category = `${severity}:${check}`;
      incrementIssueCount(counts, severity);
      categories[category] = (categories[category] ?? 0) + 1;
      entries.push({
        diagram: comparison.name ?? diagram.name,
        status: comparison.status ?? diagram.status ?? null,
        severity,
        check,
        category,
        suspected_subsystem: suspectedSubsystem(check),
        owner_task: ownerTask(check),
        message: issue.message ?? '',
        expected: issue.expected ?? issue.values?.expected ?? null,
        actual: issue.actual ?? issue.values?.actual ?? null,
        comparison_file: diagram.json_file
      });
    }
  }

  entries.sort((left, right) => {
    const severityOrder = severityRank(left.severity) - severityRank(right.severity);
    if (severityOrder !== 0) return severityOrder;
    const categoryOrder = left.category.localeCompare(right.category);
    if (categoryOrder !== 0) return categoryOrder;
    return left.diagram.localeCompare(right.diagram);
  });

  return {
    eval_dir: evalDir,
    baseline: {
      total: report.by_type?.flowchart?.total ?? report.total ?? flowchartDiagrams.length,
      matching: report.by_type?.flowchart?.matching ?? report.matching ?? 0,
      avg_structural: report.by_type?.flowchart?.avg_structural ?? null
    },
    counts,
    categories: Object.fromEntries(
      Object.entries(categories).sort((left, right) => {
        const countOrder = right[1] - left[1];
        return countOrder === 0 ? left[0].localeCompare(right[0]) : countOrder;
      })
    ),
    entries
  };
}

export function validateLedgerCounts(ledger, expected) {
  const failures = [];
  for (const [name, expectedValue] of Object.entries(expected)) {
    if (expectedValue === undefined || expectedValue === null) {
      continue;
    }
    const actualValue = ledger.counts[name] ?? 0;
    if (actualValue !== expectedValue) {
      failures.push(`${name} expected ${expectedValue}, got ${actualValue}`);
    }
  }
  return failures;
}

export function parseLedgerArgs(args) {
  let evalDir = null;
  let writePath = null;
  const expect = {};

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--write') {
      writePath = args[index + 1] ?? null;
      index += 1;
    } else if (arg === '--expect') {
      Object.assign(expect, parseExpectedCounts(args[index + 1] ?? ''));
      index += 1;
    } else if (!evalDir) {
      evalDir = arg;
    }
  }

  return {
    evalDir: evalDir ?? path.join(repoRoot, 'eval-report', 'selkie-eval-a7de3ec6'),
    writePath,
    expect
  };
}

function parseExpectedCounts(value) {
  const result = {};
  for (const part of value.split(',')) {
    const [rawKey, rawValue] = part.split('=');
    const key = rawKey?.trim();
    if (!key) continue;
    const count = Number(rawValue);
    if (!Number.isFinite(count)) {
      throw new Error(`Invalid expected count: ${part}`);
    }
    result[key] = count;
  }
  return result;
}

function incrementIssueCount(counts, severity) {
  if (severity === 'error') {
    counts.errors += 1;
  } else if (severity === 'warning') {
    counts.warnings += 1;
  } else if (severity === 'info') {
    counts.info += 1;
  } else if (severity === 'visual_only') {
    counts.visual_only += 1;
  }
}

function severityRank(severity) {
  return { error: 0, warning: 1, info: 2, visual_only: 3 }[severity] ?? 4;
}

function suspectedSubsystem(check) {
  if (['aspect_ratio', 'dimensions', 'layout_pattern', 'vertical_distribution', 'row_distribution'].includes(check)) {
    return 'layout';
  }
  if (['edge_positions', 'edge_details', 'edge_attachment_sides', 'edge_attachment_pattern', 'edge_connectivity'].includes(check)) {
    return 'edge-routing';
  }
  if (['colors', 'stroke_width', 'text_fill_colors', 'font_styles'].includes(check)) {
    return 'style';
  }
  if (['labels_missing', 'labels_extra', 'text_placement', 'text_overflow'].includes(check)) {
    return 'text';
  }
  if (['node_count', 'edge_count', 'shape_counts'].includes(check)) {
    return 'parser-or-extractor';
  }
  return 'eval-or-renderer';
}

function ownerTask(check) {
  if (['aspect_ratio', 'dimensions', 'layout_pattern', 'vertical_distribution', 'row_distribution'].includes(check)) {
    return 'phase9-layout-parity';
  }
  if (['edge_positions', 'edge_details', 'edge_attachment_sides', 'edge_attachment_pattern', 'edge_connectivity'].includes(check)) {
    return 'phase9-edge-routing-parity';
  }
  if (['colors', 'stroke_width', 'text_fill_colors', 'font_styles'].includes(check)) {
    return 'phase9-style-parity';
  }
  if (['labels_missing', 'labels_extra', 'text_placement', 'text_overflow'].includes(check)) {
    return 'phase9-label-parity';
  }
  return 'phase9-eval-hardening';
}

async function main() {
  const { evalDir, writePath, expect } = parseLedgerArgs(process.argv.slice(2));
  const ledger = await buildFlowchartIssueLedger(evalDir);
  const failures = validateLedgerCounts(ledger, expect);
  if (failures.length > 0) {
    console.error(`Phase 9 ledger count validation failed for ${evalDir}`);
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exit(1);
  }

  const json = `${JSON.stringify(ledger, null, 2)}\n`;
  if (writePath) {
    await mkdir(path.dirname(writePath), { recursive: true });
    await writeFile(writePath, json);
  } else {
    process.stdout.write(json);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
