import { readdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '../..');

export async function summarizeFlowchartEval(evalDir) {
  const reportPath = path.join(evalDir, 'report.json');
  const report = JSON.parse(await readFile(reportPath, 'utf8'));
  const flowchart = report.by_type?.flowchart ?? {};
  const issueCategories = await summarizeIssueCategories(evalDir, report);

  return {
    eval_dir: evalDir,
    total: flowchart.total ?? report.total ?? 0,
    matching: flowchart.matching ?? report.matching ?? 0,
    parity_percent: flowchart.parity_percent ?? report.parity_percent ?? 0,
    avg_structural: flowchart.avg_structural ?? null,
    issue_counts: {
      errors: report.issue_counts?.errors ?? 0,
      warnings: report.issue_counts?.warnings ?? 0,
      info: report.issue_counts?.info ?? 0,
      visual_only: report.issue_counts?.visual_only ?? 0
    },
    issue_categories: issueCategories
  };
}

export async function findLatestEvalDir(rootDir = path.join(repoRoot, 'eval-report')) {
  const candidates = await collectEvalReportDirs(rootDir);
  const latest = candidates
    .filter(Boolean)
    .sort((left, right) => right.mtimeMs - left.mtimeMs)[0];

  if (!latest) {
    throw new Error(`No selkie eval reports found under ${rootDir}`);
  }

  return latest.path;
}

async function collectEvalReportDirs(rootDir) {
  const entries = await readdir(rootDir, { withFileTypes: true });
  const candidates = [];

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const fullPath = path.join(rootDir, entry.name);
    const reportInfo = await stat(path.join(fullPath, 'report.json')).catch(() => null);
    if (entry.name.startsWith('selkie-eval-') && reportInfo) {
      candidates.push({ path: fullPath, mtimeMs: reportInfo.mtimeMs });
      continue;
    }

    const nested = await collectEvalReportDirs(fullPath);
    candidates.push(...nested);
  }

  return candidates;
}

export function parseSummaryArgs(args) {
  let evalDir = null;
  let writePath = null;

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === '--write') {
      writePath = args[index + 1] ?? null;
      index += 1;
    } else if (!evalDir) {
      evalDir = arg;
    }
  }

  return { evalDir, writePath };
}

export function checkSummaryAgainstTarget(summary, target) {
  const failures = [];

  checkMinimum(failures, 'matching', summary.matching, target.min_matching);
  checkMinimum(
    failures,
    'avg_structural',
    summary.avg_structural,
    target.min_avg_structural
  );
  checkMaximum(failures, 'errors', summary.issue_counts.errors, target.max_errors);
  checkMaximum(failures, 'warnings', summary.issue_counts.warnings, target.max_warnings);
  checkMaximum(failures, 'info', summary.issue_counts.info, target.max_info);

  for (const [category, maxCount] of Object.entries(target.max_issue_categories ?? {})) {
    checkMaximum(
      failures,
      `issue_categories.${category}`,
      summary.issue_categories[category] ?? 0,
      maxCount
    );
  }

  return failures;
}

function checkMinimum(failures, name, actual, expected) {
  if (expected === undefined || expected === null) {
    return;
  }
  if (actual < expected) {
    failures.push(`${name} expected >= ${expected}, got ${actual}`);
  }
}

function checkMaximum(failures, name, actual, expected) {
  if (expected === undefined || expected === null) {
    return;
  }
  if (actual > expected) {
    failures.push(`${name} expected <= ${expected}, got ${actual}`);
  }
}

async function summarizeIssueCategories(evalDir, report) {
  const counts = new Map();
  const flowchartDiagrams = (report.diagrams ?? []).filter(
    (diagram) => diagram.diagram_type === 'flowchart' && diagram.json_file
  );

  for (const diagram of flowchartDiagrams) {
    const comparisonPath = path.join(evalDir, diagram.json_file);
    const comparison = JSON.parse(await readFile(comparisonPath, 'utf8'));
    for (const issue of comparison.issues ?? []) {
      const key = `${issue.level}:${issue.check}`;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }

  return Object.fromEntries(
    [...counts.entries()].sort((left, right) => {
      const countOrder = right[1] - left[1];
      return countOrder === 0 ? left[0].localeCompare(right[0]) : countOrder;
    })
  );
}

async function main() {
  const args = process.argv.slice(2);
  if (args[0] === 'check-target') {
    await runTargetCheck(args.slice(1));
    return;
  }

  const { evalDir: evalDirArg, writePath } = parseSummaryArgs(args);
  const evalDir = evalDirArg ?? await findLatestEvalDir();
  const summary = await summarizeFlowchartEval(evalDir);
  const json = `${JSON.stringify(summary, null, 2)}\n`;

  if (writePath) {
    await writeFile(writePath, json);
  } else {
    process.stdout.write(json);
  }
}

async function runTargetCheck(args) {
  const targetPath = args[0];
  if (!targetPath) {
    throw new Error('Usage: flowchart-eval-summary.mjs check-target <target.json> [eval-dir]');
  }

  const { evalDir: evalDirArg } = parseSummaryArgs(args.slice(1));
  const evalDir = evalDirArg ?? await findLatestEvalDir();
  const target = JSON.parse(await readFile(targetPath, 'utf8'));
  const summary = await summarizeFlowchartEval(evalDir);
  const failures = checkSummaryAgainstTarget(summary, target);

  if (failures.length > 0) {
    console.error(`Flowchart eval target failed for ${evalDir}`);
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exit(1);
  }

  console.log(`Flowchart eval target passed for ${evalDir}`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
