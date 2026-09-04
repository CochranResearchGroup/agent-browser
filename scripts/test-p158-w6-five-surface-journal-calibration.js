#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  createP158W6MalformedLineSeamReceipt,
  executeP158W6FiveSurfaceJournalCalibration,
  P158_W6_FAILURE_CATEGORIES,
  P158W6JournalCalibrationError,
} from './lib/p158-w6-five-surface-journal-calibration.js';
import {
  createDevelopmentBrowserManagerInducer,
  runP158W6JournalCalibrationCli,
} from './run-p158-w6-five-surface-journal-calibration.js';

const NOW = '2026-09-04T22:01:00.000Z';
const CANDIDATE = {
  candidateSha256: '11'.repeat(32), executableSha256: '22'.repeat(32),
  dashboardSha256: '33'.repeat(32), installedGenerationId: 'development-generation-p158',
  packageVersion: '0.28.0-development', serviceContractVersion: 'service-ui-runtime.v1',
};
const ENVIRONMENT = {
  environmentId: 'E2', runtimeLane: 'development', production: false,
  dashboardOrigin: 'https://development-dashboard.example.test',
};

function journalRecord({ occurrenceId, category, code, action, stage = 'p158_w6_calibration', details }) {
  return {
    schemaVersion: 'agent-browser.service-failure-record.v1', occurrenceId,
    occurredAt: NOW, bootEpoch: 'boot-p158', runtimeEnvironment: 'development',
    category, source: category === 'browser_launch' ? 'browser_manager' : 'authenticated_dashboard_client',
    stage, code, summary: 'Synthetic development calibration failure.', action,
    references: {},
    ...(details ? { details } : {}),
  };
}

const MALFORMED_READBACK = {
  schemaVersion: 'agent-browser.service-failure-journal-readback.v1',
  records: [
    journalRecord({ occurrenceId: 'isolated-before', category: 'dashboard_action',
      code: 'before-malformed', action: 'isolated_read' }),
    journalRecord({ occurrenceId: 'isolated-after', category: 'dashboard_action',
      code: 'after-malformed', action: 'isolated_read' }),
  ],
  malformedLineCount: 1, writeFailureCount: 0,
};

function malformedReceipt() {
  return createP158W6MalformedLineSeamReceipt({
    candidateSha256: CANDIDATE.candidateSha256, isolationId: 'disposable-journal-p158',
    readback: MALFORMED_READBACK, beforeCode: 'before-malformed', afterCode: 'after-malformed',
    clock: () => NOW,
  });
}

function input(overrides = {}) {
  return {
    runId: 'p158-w6-journal-calibration', candidate: structuredClone(CANDIDATE),
    environment: structuredClone(ENVIRONMENT),
    window: { notBefore: '2026-09-04T22:00:00.000Z', notAfter: '2026-09-04T22:05:00.000Z' },
    malformedLineReceipt: malformedReceipt(), clock: () => NOW, ...overrides,
  };
}

function response(url, status, body) {
  return {
    url: String(url), status, ok: status >= 200 && status < 300, redirected: false,
    async json() { return structuredClone(body); },
  };
}

function fixtureFetch({ authenticated = true, wrongManifest = false, omitCategory = null,
  duplicateCategory = null, legacyParserOnly = false } = {}) {
  const calls = [];
  const records = [];
  let baselineRead = false;
  let calibrationKey;
  const fetch = async (url, options = {}) => {
    const parsed = new URL(url);
    calls.push({ url: parsed.href, method: options.method ?? 'GET', body: options.body ?? null });
    if (parsed.pathname === '/api/dashboard-auth/status') {
      return response(url, 200, { authenticated });
    }
    if (parsed.pathname === '/api/runtime/manifest') {
      return response(url, 200, {
        schemaVersion: 'agent-browser.runtime-manifest.v1', runtimeEnvironment: 'development',
        executable: { sha256: wrongManifest ? '44'.repeat(32) : CANDIDATE.executableSha256 },
        dashboard: { sha256: CANDIDATE.dashboardSha256 }, packageVersion: CANDIDATE.packageVersion,
        serviceContractVersion: CANDIDATE.serviceContractVersion,
      });
    }
    if (parsed.pathname === '/api/service/failures') {
      if (!baselineRead) {
        baselineRead = true;
        return response(url, 200, { success: true, data: {
          schemaVersion: 'agent-browser.service-failure-journal-readback.v1', records: [],
          malformedLineCount: 0, writeFailureCount: 0,
        } });
      }
      const visible = omitCategory ? records.filter((record) => record.category !== omitCategory) : records;
      const duplicate = duplicateCategory
        ? visible.find((record) => record.category === duplicateCategory)
        : null;
      const finalRecords = duplicate ? [...visible, structuredClone(duplicate)] : visible;
      if (duplicate) finalRecords.at(-1).occurrenceId += '-duplicate';
      return response(url, 200, { success: true, data: {
        schemaVersion: 'agent-browser.service-failure-journal-readback.v1', records: finalRecords,
        malformedLineCount: 0, writeFailureCount: 0,
      } });
    }
    if (parsed.pathname === '/api/service/failure-observation') {
      const observation = JSON.parse(options.body);
      assert(P158_W6_FAILURE_CATEGORIES.includes(observation.category));
      assert.notEqual(observation.category, 'browser_launch');
      assert.equal(observation.code, `p158_w6_${observation.category}_${calibrationKey}`);
      const occurrenceId = `${observation.category}-occurrence`;
      records.push(journalRecord({ occurrenceId, category: observation.category,
        code: observation.code, action: observation.action }));
      return response(url, 202, { success: true, data: { occurrenceId, recorded: true } });
    }
    throw new Error(`Unexpected URL: ${url}`);
  };
  const induceBrowserLaunchFailure = async ({ calibrationKey: key, candidate, environment }) => {
    calibrationKey = key;
    const engine = `p158-invalid-${key}`;
    records.push(legacyParserOnly
      ? {
          ...journalRecord({ occurrenceId: 'browser-launch-occurrence', category: 'browser_launch',
            code: 'browser_launch_failed', action: 'cdp_free_launch', stage: 'failed' }),
          source: 'service_control_plane', references: { sessionId: `p158-w6-journal-${key}` },
          details: { effectState: 'no_effect' },
        }
      : journalRecord({ occurrenceId: 'browser-launch-occurrence', category: 'browser_launch',
          code: 'browser_launch_failed', action: 'open', stage: 'launch',
          details: { engine, profileConfigured: false, headed: false } }));
    return {
      schemaVersion: 'agent-browser.p158-w6-browser-manager-induction.v1',
      runtimeEnvironment: environment.runtimeLane,
      candidateSha256: candidate.candidateSha256,
      executableSha256: candidate.executableSha256,
      installedGenerationIdSha256: sha256(candidate.installedGenerationId),
      engine,
      browserManagerLaunchInvoked: !legacyParserOnly,
      browserProcessSpawnAttempted: false,
      resultState: 'failed_as_expected',
      startedAt: NOW,
      completedAt: NOW,
      retryAttempted: false,
      repairAttempted: false,
    };
  };
  return { fetch, calls, induceBrowserLaunchFailure };
}

async function runTest(name, body) {
  try {
    await body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

await runTest('seals malformed-line recovery only from an isolated readback seam', () => {
  const receipt = malformedReceipt();
  assert.equal(receipt.isolatedRuntimeState, true);
  assert.equal(receipt.liveJournalMutated, false);
  assert.equal(receipt.malformedLineCount, 1);
  const { receiptSha256, ...body } = receipt;
  assert.equal(receiptSha256, sha256(body));
  assert.throws(() => createP158W6MalformedLineSeamReceipt({
    candidateSha256: CANDIDATE.candidateSha256, isolationId: 'bad-seam',
    readback: { ...MALFORMED_READBACK, malformedLineCount: 0 },
    beforeCode: 'before-malformed', afterCode: 'after-malformed', clock: () => NOW,
  }), (error) => error instanceof P158W6JournalCalibrationError &&
    error.code === 'w6_malformed_line_recovery_missing');
});

await runTest('records exactly the five named development failure surfaces', async () => {
  const fixture = fixtureFetch();
  const artifact = await executeP158W6FiveSurfaceJournalCalibration({
    ...input(), fetch: fixture.fetch, induceBrowserLaunchFailure: fixture.induceBrowserLaunchFailure,
    sleep: async () => {},
  });
  assert.deepEqual(artifact.categories, P158_W6_FAILURE_CATEGORIES);
  assert.deepEqual(artifact.records.map((record) => record.category), P158_W6_FAILURE_CATEGORIES);
  assert.equal(artifact.requestedFailureCount, 5);
  assert.equal(artifact.observedFailureCount, 5);
  assert.equal(artifact.authenticatedDashboardSession, true);
  assert.equal(artifact.productionAccessAllowed, false);
  assert.equal(artifact.liveJournalMalformedLineInjected, false);
  assert.equal(artifact.browserManagerLaunchInvoked, true);
  assert.equal(artifact.browserProcessSpawnAttempted, false);
  assert.equal(fixture.calls.length, 10);
  assert.equal(fixture.calls.filter((call) => call.method === 'POST').length, 4);
  assert(fixture.calls.every((call) => new URL(call.url).origin === ENVIRONMENT.dashboardOrigin));
  assert(!JSON.stringify(artifact).includes(ENVIRONMENT.dashboardOrigin));
  assert(!JSON.stringify(artifact).includes('sessionId'));
  const { artifactSha256, ...body } = artifact;
  assert.equal(artifactSha256, sha256(body));
});

await runTest('fails closed before effects for production, authentication, candidate, time, and seam drift', async () => {
  const cases = [
    ['w6_journal_identity_unproven', { environment: { ...ENVIRONMENT, production: true } }, fixtureFetch()],
    ['w6_journal_time_window_invalid', { window: {
      notBefore: '2026-09-04T21:00:00.000Z', notAfter: '2026-09-04T21:05:00.000Z',
    } }, fixtureFetch()],
    ['w6_malformed_line_evidence_invalid', { malformedLineReceipt: {
      ...malformedReceipt(), candidateSha256: '55'.repeat(32),
    } }, fixtureFetch()],
    ['w6_journal_authentication_required', {}, fixtureFetch({ authenticated: false })],
    ['w6_journal_candidate_mismatch', {}, fixtureFetch({ wrongManifest: true })],
  ];
  for (const [code, overrides, fixture] of cases) {
    await assert.rejects(
      executeP158W6FiveSurfaceJournalCalibration({
        ...input(overrides), fetch: fixture.fetch,
        induceBrowserLaunchFailure: fixture.induceBrowserLaunchFailure, sleep: async () => {},
      }),
      (error) => error instanceof P158W6JournalCalibrationError && error.code === code,
    );
    assert.equal(fixture.calls.filter((call) => call.method === 'POST').length, 0, code);
  }
});

await runTest('detects missing and duplicate journal correlations without retry or repair', async () => {
  for (const fixture of [fixtureFetch({ omitCategory: 'handoff_link' }),
    fixtureFetch({ duplicateCategory: 'cdp_stream' })]) {
    await assert.rejects(
      executeP158W6FiveSurfaceJournalCalibration({
        ...input(), fetch: fixture.fetch,
        induceBrowserLaunchFailure: fixture.induceBrowserLaunchFailure, sleep: async () => {},
      }),
      (error) => error instanceof P158W6JournalCalibrationError &&
        error.code === 'w6_five_surface_correlation_invalid',
    );
    assert.equal(fixture.calls.filter((call) => call.method === 'POST').length, 4);
  }
});

await runTest('rejects the old cdp-free parser-only false browser-launch proof', async () => {
  const fixture = fixtureFetch({ legacyParserOnly: true });
  await assert.rejects(
    executeP158W6FiveSurfaceJournalCalibration({
      ...input(), fetch: fixture.fetch,
      induceBrowserLaunchFailure: fixture.induceBrowserLaunchFailure, sleep: async () => {},
    }),
    (error) => error instanceof P158W6JournalCalibrationError &&
      error.code === 'w6_browser_manager_induction_invalid',
  );
  assert.equal(fixture.calls.filter((call) => call.method === 'POST').length, 0);
});

await runTest('binds the live inducer to the installed development generation and invalid engine', async () => {
  const binary = Buffer.from('provider-free-development-candidate');
  const executableSha256 = createHash('sha256').update(binary).digest('hex');
  const candidate = { ...CANDIDATE, executableSha256, installedGenerationId: 'generation-p158' };
  let invocation;
  const inducer = await createDevelopmentBrowserManagerInducer({
    env: { SAFE_PARENT: 'preserved', AGENT_BROWSER_PROFILE: 'must-not-leak' },
    clock: () => NOW,
    descriptor: {
      current: '/development/current', pseudoHome: '/development/home',
      socketDir: '/development/socket',
      runtimeHostIngressState: '/development/home/.agent-browser/runtime-host-ingress.json',
    },
    realpathImpl: async () => '/development/generations/generation-p158',
    readFileImpl: async (path) => {
      assert.equal(path, '/development/generations/generation-p158/bin/agent-browser');
      return binary;
    },
    runProcess: async (command, args, options) => {
      invocation = { command, args, options };
      return { code: 0, signal: null, stderr: '', stdout: JSON.stringify({
        success: false, error: "Unknown engine 'p158-invalid-test'. Supported engines: chrome, lightpanda",
      }) };
    },
  });
  const receipt = await inducer({
    calibrationKey: 'test', engine: 'p158-invalid-test', candidate,
    environment: ENVIRONMENT,
  });
  assert.deepEqual(invocation.args, [
    '--json', '--session', 'p158-w6-journal-test', '--engine', 'p158-invalid-test',
    'open', 'about:blank',
  ]);
  assert.equal(invocation.command, '/development/generations/generation-p158/bin/agent-browser');
  assert.equal(invocation.options.env.HOME, '/development/home');
  assert.equal(invocation.options.env.AGENT_BROWSER_RUNTIME_ENVIRONMENT, 'development');
  assert.equal(invocation.options.env.AGENT_BROWSER_SOCKET_DIR, '/development/socket');
  assert.equal(invocation.options.env.AGENT_BROWSER_PROFILE, undefined);
  assert.equal(receipt.executableSha256, executableSha256);
  assert.equal(receipt.browserManagerLaunchInvoked, true);
  assert.equal(receipt.browserProcessSpawnAttempted, false);
});

await runTest('CLI writes one private hash-bound artifact without printing sensitive inputs', async () => {
  const root = await mkdtemp(join(tmpdir(), 'p158-w6-journal-cli-'));
  try {
    const configPath = join(root, 'config.json');
    const malformedPath = join(root, 'malformed.json');
    const outputPath = join(root, 'artifact.json');
    await writeFile(configPath, JSON.stringify(input({ malformedLineReceipt: undefined, fetch: undefined })));
    await writeFile(malformedPath, JSON.stringify(malformedReceipt()));
    const fixture = fixtureFetch();
    let stdout = '';
    const artifact = await runP158W6JournalCalibrationCli([
      '--config', configPath, '--malformed-line-receipt', malformedPath,
      '--auth-env', join(root, 'not-read.env'), '--output', outputPath,
    ], {
      authenticatedFetch: fixture.fetch, induceBrowserLaunchFailure: fixture.induceBrowserLaunchFailure,
      sleep: async () => {}, clock: () => NOW,
      stdout: { write(value) { stdout += value; } },
    });
    const persisted = JSON.parse(await readFile(outputPath, 'utf8'));
    assert.deepEqual(persisted, artifact);
    assert.equal(JSON.parse(stdout).state, 'passed');
    assert(!stdout.includes(ENVIRONMENT.dashboardOrigin));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

process.stdout.write('P158 W6 five-surface journal calibration tests passed\n');
