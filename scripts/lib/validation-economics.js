const JOB_NAMES = Object.freeze({
  docs: 'Documentation',
  versionSync: 'Version Sync Check',
  rustQuality: 'Rust Quality',
  rust: 'Rust',
  dashboard: 'Dashboard',
  serviceClient: 'Service Client',
  workstation: 'Workstation Fixtures',
  comprehensive: 'Comprehensive Rust',
});

export function summarizeValidationEconomics({ selection, jobs }) {
  const selectedNames = ['Validation Selection', ...Object.entries(selection.jobs ?? {})
    .filter(([, selected]) => selected)
    .map(([key]) => JOB_NAMES[key])
    .filter(Boolean)];
  const completed = jobs.filter((job) => selectedNames.includes(job.name) && job.completed_at);
  const missing = selectedNames.filter((name) => !completed.some((job) => job.name === name));
  if (missing.length > 0) throw new Error(`selected jobs missing completed timing: ${missing.join(', ')}`);

  const intervals = completed.map((job) => ({
    name: job.name,
    seconds: Math.max(0, (Date.parse(job.completed_at) - Date.parse(job.started_at)) / 1000),
    startedAt: job.started_at,
    completedAt: job.completed_at,
  }));
  const started = intervals.map((entry) => Date.parse(entry.startedAt));
  const completedAt = intervals.map((entry) => Date.parse(entry.completedAt));
  const observedRunnerSeconds = intervals.reduce((sum, entry) => sum + entry.seconds, 0);
  const wallSeconds = intervals.length === 0 ? 0 : (Math.max(...completedAt) - Math.min(...started)) / 1000;
  return {
    schemaVersion: 'agent-browser.validation-economics.v1',
    tier: selection.tier,
    selectedJobs: selectedNames,
    exclusions: selection.exclusions ?? [],
    wallSeconds,
    observedRunnerSeconds,
    observedRunnerMinutes: Number((observedRunnerSeconds / 60).toFixed(2)),
    jobs: intervals,
    note: 'Observed duration for selected jobs through aggregate start; excludes Presubmit itself and separate slow qualification jobs, and is not billing-exact runner usage.',
  };
}

export function economicsMarkdown(summary) {
  return [
    '## Validation economics',
    '',
    `- Tier: \`${summary.tier}\``,
    `- Selected jobs: ${summary.selectedJobs.join(', ') || 'none'}`,
    `- Selected-job wall time through aggregate start: ${summary.wallSeconds} seconds`,
    `- Selected-job observed runner time: ${summary.observedRunnerMinutes} minutes`,
    `- Exclusions: ${summary.exclusions.join('; ') || 'none'}`,
    `- Note: ${summary.note}`,
    '',
  ].join('\n');
}
