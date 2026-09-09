#!/usr/bin/env node

import { runP158FinalAnalysis } from './lib/p158-final-analysis-runner.js';

function usage() {
  return 'Usage: node scripts/run-p158-final-analysis.js --descriptor <absolute-path> --descriptor-sha256 <sha256>\n';
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

if (process.argv.includes('--help')) {
  process.stdout.write(usage());
  process.exit(0);
}

try {
  const result = await runP158FinalAnalysis({
    descriptorPath: argument('--descriptor'),
    descriptorSha256: argument('--descriptor-sha256'),
  });
  process.stdout.write(`${JSON.stringify({
    state: 'analyzed', runId: result.report.runId, resumed: result.resumed,
    reportSha256: result.report.reportSha256,
    reviewSha256: result.reviewCandidate.reviewSha256,
    outputPaths: result.outputPaths,
    effectsAttempted: false, repairAttempted: false,
  })}\n`);
} catch (error) {
  process.stderr.write(`${JSON.stringify({
    state: 'failed', code: error?.code ?? 'p158_final_analysis_failed',
    message: error?.message ?? String(error), effectsAttempted: false, repairAttempted: false,
  })}\n`);
  process.exitCode = 1;
}
