const SCHEMA_VERSION = 'agent-browser.validation-selection.v2';

export const PRESUBMIT_JOB_KEYS = Object.freeze([
  'docs',
  'versionSync',
  'rustQuality',
  'rust',
  'dashboard',
  'serviceClient',
  'workstation',
]);

const TERMINAL_RESULTS = new Set(['success', 'failure', 'cancelled', 'skipped']);

export function verifyPresubmit({ selection, results }) {
  if (selection?.schemaVersion !== SCHEMA_VERSION) {
    throw new Error(`unsupported validation selection schema: ${selection?.schemaVersion ?? 'missing'}`);
  }
  if (!selection.jobs || typeof selection.jobs !== 'object' || Array.isArray(selection.jobs)) {
    throw new Error('validation selection jobs must be an object');
  }
  if (!results || typeof results !== 'object' || Array.isArray(results)) {
    throw new Error('presubmit job results must be an object');
  }

  const unexpectedJobs = Object.keys(selection.jobs).filter((key) => !PRESUBMIT_JOB_KEYS.includes(key));
  if (unexpectedJobs.length > 0) {
    throw new Error(`validation selection contains unknown jobs: ${unexpectedJobs.sort().join(', ')}`);
  }

  const selected = [];
  const excluded = [];
  for (const key of PRESUBMIT_JOB_KEYS) {
    const expected = selection.jobs[key];
    if (typeof expected !== 'boolean') {
      throw new Error(`validation selection job ${key} must be boolean`);
    }
    const result = results[key];
    if (!TERMINAL_RESULTS.has(result)) {
      throw new Error(`presubmit job ${key} has unexpected result: ${result ?? 'missing'}`);
    }
    if (expected) {
      selected.push(key);
      if (result !== 'success') {
        throw new Error(`selected presubmit job ${key} ended with ${result}`);
      }
    } else {
      excluded.push(key);
      if (result !== 'skipped') {
        throw new Error(`unselected presubmit job ${key} ended with ${result}`);
      }
    }
  }

  return Object.freeze({
    schemaVersion: SCHEMA_VERSION,
    tier: selection.tier,
    selected,
    excluded,
    unknownFiles: [...(selection.unknownFiles ?? [])],
  });
}
