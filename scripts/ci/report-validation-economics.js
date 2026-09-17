#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';

import { economicsMarkdown, summarizeValidationEconomics } from '../lib/validation-economics.js';

try {
  const selection = JSON.parse(requireEnvironment('VALIDATION_SELECTION_JSON'));
  const payload = JSON.parse(readFileSync(requireEnvironment('VALIDATION_JOBS_PATH'), 'utf8'));
  const summary = summarizeValidationEconomics({ selection, jobs: payload.jobs ?? payload });
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    writeFileSync(process.env.GITHUB_STEP_SUMMARY, economicsMarkdown(summary), { flag: 'a' });
  }
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}

function requireEnvironment(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}
