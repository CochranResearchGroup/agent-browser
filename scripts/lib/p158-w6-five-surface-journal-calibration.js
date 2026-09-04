import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { sha256 } from './p158-campaign-controller.js';

export const P158_W6_JOURNAL_CALIBRATION_SOURCE_PATH =
  'scripts/lib/p158-w6-five-surface-journal-calibration.js';

export const P158_W6_FAILURE_CATEGORIES = Object.freeze([
  'browser_launch',
  'guacamole_load',
  'handoff_link',
  'cdp_stream',
  'dashboard_action',
]);

const CLIENT_CATEGORIES = P158_W6_FAILURE_CATEGORIES.slice(1);
const SHA256 = /^[a-f0-9]{64}$/u;
const RECORD_SCHEMA = 'agent-browser.service-failure-record.v1';
const READBACK_SCHEMA = 'agent-browser.service-failure-journal-readback.v1';
const MAX_WINDOW_MS = 15 * 60_000;
const QUIET_WINDOW_READS = 2;
const MAX_QUIET_WINDOW_ATTEMPTS = 6;

export class P158W6JournalCalibrationError extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158W6JournalCalibrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W6JournalCalibrationError(code, message, details);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function exactOrigin(value) {
  let parsed;
  try { parsed = new URL(value); } catch {
    fail('w6_journal_origin_invalid', 'W6 journal calibration requires one exact HTTP dashboard origin');
  }
  if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password ||
      parsed.pathname !== '/' || parsed.search || parsed.hash) {
    fail('w6_journal_origin_invalid', 'W6 journal calibration requires one exact HTTP dashboard origin');
  }
  return parsed.origin;
}

function validateWindow(window, clock) {
  const notBefore = Date.parse(window?.notBefore ?? '');
  const notAfter = Date.parse(window?.notAfter ?? '');
  const now = Date.parse(clock());
  if (!Number.isFinite(notBefore) || !Number.isFinite(notAfter) || !Number.isFinite(now) ||
      notAfter <= notBefore || notAfter - notBefore > MAX_WINDOW_MS || now < notBefore || now > notAfter) {
    fail('w6_journal_time_window_invalid', 'W6 journal calibration is outside its bounded execution window');
  }
  return { notBefore: new Date(notBefore).toISOString(), notAfter: new Date(notAfter).toISOString() };
}

function validateReadback(payload, label) {
  const readback = payload?.success === true ? payload.data : payload;
  if (readback?.schemaVersion !== READBACK_SCHEMA || !Array.isArray(readback.records) ||
      !Number.isInteger(readback.malformedLineCount) || readback.malformedLineCount < 0 ||
      !Number.isInteger(readback.writeFailureCount) || readback.writeFailureCount < 0 ||
      readback.records.some((record) => record?.schemaVersion !== RECORD_SCHEMA)) {
    fail('w6_journal_readback_invalid', `${label} is not a valid failure-journal readback`);
  }
  return structuredClone(readback);
}

async function fetchJson(fetchImpl, url, init, label) {
  let response;
  try { response = await fetchImpl(url, init); } catch (error) {
    fail('w6_journal_transport_failed', `${label} failed before an HTTP response`, {
      errorName: error?.name ?? 'Error',
    });
  }
  if (!response || !Number.isInteger(response.status) || typeof response.json !== 'function') {
    fail('w6_journal_transport_invalid', `${label} did not return an HTTP response`);
  }
  if (response.redirected === true || (response.url && new URL(response.url).href !== new URL(url).href)) {
    fail('w6_journal_redirect_forbidden', `${label} redirected away from the sealed development origin`);
  }
  let body;
  try { body = await response.json(); } catch {
    fail('w6_journal_response_invalid', `${label} did not return JSON`);
  }
  return { response, body };
}

function validateMalformedLineReceipt(receipt, candidateSha256) {
  if (receipt?.schemaVersion !== 'agent-browser.p158-w6-malformed-line-seam-receipt.v1' ||
      receipt.candidateSha256 !== candidateSha256 || receipt.runtimeEnvironment !== 'development' ||
      receipt.isolatedRuntimeState !== true || receipt.liveJournalMutated !== false ||
      receipt.malformedLineCount < 1 || receipt.validRecordBeforeMalformed !== true ||
      receipt.validRecordAfterMalformed !== true || receipt.resultState !== 'passed' ||
      receipt.receiptSha256 !== sha256(without(receipt, 'receiptSha256'))) {
    fail('w6_malformed_line_evidence_invalid',
      'W6 requires a self-hashed, isolated malformed-line readback receipt for the same candidate');
  }
  return structuredClone(receipt);
}

/**
 * Seal evidence from an isolated readback seam. The caller may corrupt only a
 * disposable journal; the installed development journal is never modified.
 */
export function createP158W6MalformedLineSeamReceipt({
  candidateSha256, isolationId, readback, beforeCode, afterCode,
  clock = () => new Date().toISOString(),
}) {
  if (!SHA256.test(candidateSha256 ?? '') || typeof isolationId !== 'string' || !isolationId ||
      typeof beforeCode !== 'string' || !beforeCode || typeof afterCode !== 'string' || !afterCode) {
    fail('w6_malformed_line_input_invalid', 'Malformed-line calibration requires exact isolated inputs');
  }
  const normalized = validateReadback(readback, 'Isolated malformed-line readback');
  const beforeIndex = normalized.records.findIndex((record) => record.code === beforeCode);
  const afterIndex = normalized.records.findIndex((record) => record.code === afterCode);
  if (normalized.malformedLineCount < 1 || beforeIndex < 0 || afterIndex <= beforeIndex) {
    fail('w6_malformed_line_recovery_missing',
      'Isolated readback must preserve valid records on both sides of at least one malformed line');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-w6-malformed-line-seam-receipt.v1',
    planId: 'P158', candidateSha256, runtimeEnvironment: 'development',
    isolationIdSha256: sha256(isolationId), isolatedRuntimeState: true,
    liveJournalMutated: false, malformedLineCount: normalized.malformedLineCount,
    validRecordBeforeMalformed: true, validRecordAfterMalformed: true,
    readbackSha256: sha256(normalized), resultState: 'passed', observedAt: clock(),
  };
  return freeze({ ...body, receiptSha256: sha256(body) });
}

function validateInput({
  runId, candidate, environment, window, malformedLineReceipt, fetch, clock,
  induceBrowserLaunchFailure, sleep,
}) {
  if (typeof runId !== 'string' || !runId || !SHA256.test(candidate?.candidateSha256 ?? '') ||
      !SHA256.test(candidate?.executableSha256 ?? '') || !SHA256.test(candidate?.dashboardSha256 ?? '') ||
      typeof candidate?.installedGenerationId !== 'string' || !candidate.installedGenerationId ||
      environment?.environmentId !== 'E2' || environment.runtimeLane !== 'development' ||
      environment.production !== false || typeof fetch !== 'function' || typeof clock !== 'function' ||
      typeof induceBrowserLaunchFailure !== 'function' || typeof sleep !== 'function') {
    fail('w6_journal_identity_unproven',
      'W6 journal calibration requires the exact E2 development candidate and injected dependencies');
  }
  return {
    origin: exactOrigin(environment.dashboardOrigin),
    boundedWindow: validateWindow(window, clock),
    malformedLineReceipt: validateMalformedLineReceipt(malformedLineReceipt, candidate.candidateSha256),
  };
}

function validateBrowserManagerInduction(receipt, { candidate, engine, boundedWindow }) {
  const startedAt = Date.parse(receipt?.startedAt ?? '');
  const completedAt = Date.parse(receipt?.completedAt ?? '');
  if (receipt?.schemaVersion !== 'agent-browser.p158-w6-browser-manager-induction.v1' ||
      receipt.runtimeEnvironment !== 'development' ||
      receipt.candidateSha256 !== candidate.candidateSha256 ||
      receipt.executableSha256 !== candidate.executableSha256 ||
      receipt.installedGenerationIdSha256 !== sha256(candidate.installedGenerationId) ||
      receipt.engine !== engine || receipt.browserManagerLaunchInvoked !== true ||
      receipt.browserProcessSpawnAttempted !== false || receipt.resultState !== 'failed_as_expected' ||
      receipt.retryAttempted !== false || receipt.repairAttempted !== false ||
      !Number.isFinite(startedAt) || !Number.isFinite(completedAt) || completedAt < startedAt ||
      startedAt < Date.parse(boundedWindow.notBefore) ||
      completedAt > Date.parse(boundedWindow.notAfter)) {
    fail('w6_browser_manager_induction_invalid',
      'W6 browser-launch evidence did not prove one candidate-bound BrowserManager failure before process spawn');
  }
  return structuredClone(receipt);
}

function manifestMatches(manifest, candidate) {
  return manifest?.schemaVersion === 'agent-browser.runtime-manifest.v1' &&
    manifest.runtimeEnvironment === 'development' &&
    manifest.executable?.sha256 === candidate.executableSha256 &&
    manifest.dashboard?.sha256 === candidate.dashboardSha256 &&
    (!candidate.packageVersion || manifest.packageVersion === candidate.packageVersion) &&
    (!candidate.serviceContractVersion || manifest.serviceContractVersion === candidate.serviceContractVersion);
}

function newRecords(readback, baselineIds) {
  return readback.records.filter((record) => !baselineIds.has(record.occurrenceId));
}

function isExactBrowserManagerFailure(record, engine) {
  return record?.runtimeEnvironment === 'development' && record.category === 'browser_launch' &&
    record.source === 'browser_manager' && record.stage === 'launch' &&
    record.code === 'browser_launch_failed' && record.action === 'open' &&
    record.details?.engine === engine && record.details?.profileConfigured === false &&
    record.details?.headed === false && Object.keys(record.references ?? {}).length === 0;
}

async function readQuietBrowserManagerDelta({
  fetch, origin, common, baselineIds, engine, sleep,
}) {
  let stableSha256 = null;
  let stableReads = 0;
  for (let attempt = 0; attempt < MAX_QUIET_WINDOW_ATTEMPTS; attempt += 1) {
    const result = await fetchJson(fetch, new URL('/api/service/failures?limit=1000', origin), common,
      'BrowserManager failure quiet-window readback');
    if (!result.response.ok) {
      fail('w6_journal_read_failed', 'BrowserManager failure quiet-window readback returned a non-success response');
    }
    const readback = validateReadback(result.body, 'BrowserManager failure quiet-window readback');
    const delta = newRecords(readback, baselineIds);
    if (delta.length > 1 || (delta.length === 1 && !isExactBrowserManagerFailure(delta[0], engine))) {
      fail('w6_browser_manager_induction_invalid',
        'W6 quiet-window delta was not exactly the induced BrowserManager launch failure');
    }
    if (delta.length === 1) {
      const currentSha256 = sha256(delta);
      stableReads = currentSha256 === stableSha256 ? stableReads + 1 : 1;
      stableSha256 = currentSha256;
      if (stableReads === QUIET_WINDOW_READS) return { readback, record: delta[0], stableReads };
    }
    if (attempt + 1 < MAX_QUIET_WINDOW_ATTEMPTS) await sleep(25);
  }
  fail('w6_browser_manager_induction_invalid',
    'W6 could not observe one stable BrowserManager launch failure during the bounded quiet window');
}

function safeProjection(record) {
  return {
    category: record.category,
    source: record.source,
    stage: record.stage,
    code: record.code,
    action: record.action ?? null,
    occurredAt: record.occurredAt,
    occurrenceIdSha256: sha256(record.occurrenceId),
    referencesSha256: sha256(record.references ?? {}),
  };
}

/** Execute one no-launch server failure and four authenticated client observations. */
export async function executeP158W6FiveSurfaceJournalCalibration({
  runId, candidate, environment, window, malformedLineReceipt,
  fetch = globalThis.fetch, clock = () => new Date().toISOString(),
  induceBrowserLaunchFailure,
  sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
} = {}) {
  const validated = validateInput({
    runId, candidate, environment, window, malformedLineReceipt, fetch, clock,
    induceBrowserLaunchFailure, sleep,
  });
  const common = { method: 'GET', redirect: 'error', cache: 'no-store', headers: { accept: 'application/json' } };
  const auth = await fetchJson(fetch, new URL('/api/dashboard-auth/status', validated.origin), common,
    'Dashboard authentication check');
  const authenticated = auth.body?.authenticated === true || auth.body?.data?.authenticated === true;
  if (!auth.response.ok || !authenticated) {
    fail('w6_journal_authentication_required', 'W6 journal calibration requires an authenticated dashboard session');
  }
  const manifestResult = await fetchJson(fetch, new URL('/api/runtime/manifest', validated.origin), common,
    'Development runtime manifest');
  if (!manifestResult.response.ok || !manifestMatches(manifestResult.body, candidate)) {
    fail('w6_journal_candidate_mismatch', 'The authenticated dashboard does not expose the sealed development candidate');
  }
  const beforeResult = await fetchJson(fetch, new URL('/api/service/failures?limit=1000', validated.origin), common,
    'Failure journal baseline');
  if (!beforeResult.response.ok) fail('w6_journal_read_failed', 'Failure journal baseline returned a non-success response');
  const before = validateReadback(beforeResult.body, 'Failure journal baseline');
  const baselineIds = new Set(before.records.map((record) => record.occurrenceId));
  const calibrationKey = sha256({ runId, candidateSha256: candidate.candidateSha256 }).slice(0, 16);
  const engine = `p158-invalid-${calibrationKey}`;
  let rawInduction;
  try {
    rawInduction = await induceBrowserLaunchFailure({
      runId, calibrationKey, engine, candidate: structuredClone(candidate),
      environment: structuredClone(environment), window: structuredClone(validated.boundedWindow),
    });
  } catch (error) {
    fail('w6_browser_manager_induction_invalid',
      'W6 BrowserManager failure induction did not complete as specified', {
        errorName: error?.name ?? 'Error',
      });
  }
  const induction = validateBrowserManagerInduction(rawInduction, {
    candidate, engine, boundedWindow: validated.boundedWindow,
  });
  const quiet = await readQuietBrowserManagerDelta({
    fetch, origin: validated.origin, common, baselineIds, engine, sleep,
  });
  const submittedOccurrenceIds = new Map();
  for (const category of CLIENT_CATEGORIES) {
    const observation = {
      category, stage: 'p158_w6_calibration', code: `p158_w6_${category}_${calibrationKey}`,
      summary: 'Synthetic development calibration failure.', action: 'journal_calibration',
      observationId: `p158-w6-${category}-${calibrationKey}`,
    };
    const submitted = await fetchJson(fetch, new URL('/api/service/failure-observation', validated.origin), {
      method: 'POST', redirect: 'error', headers: { 'content-type': 'application/json' },
      body: JSON.stringify(observation),
    }, `Failure observation ${category}`);
    const occurrenceId = submitted.body?.data?.occurrenceId;
    if (submitted.response.status !== 202 || submitted.body?.success !== true ||
        typeof occurrenceId !== 'string' || !occurrenceId) {
      fail('w6_client_observation_rejected', `Authenticated ${category} observation was not durably accepted`);
    }
    submittedOccurrenceIds.set(category, occurrenceId);
  }
  if (Date.parse(clock()) > Date.parse(validated.boundedWindow.notAfter)) {
    fail('w6_journal_time_window_expired', 'W6 journal calibration exceeded its bounded execution window');
  }
  const afterResult = await fetchJson(fetch, new URL('/api/service/failures?limit=1000', validated.origin), common,
    'Failure journal final readback');
  if (!afterResult.response.ok) fail('w6_journal_read_failed', 'Failure journal final readback returned a non-success response');
  const after = validateReadback(afterResult.body, 'Failure journal final readback');
  const delta = newRecords(after, baselineIds);
  const launchMatches = delta.filter((record) => isExactBrowserManagerFailure(record, engine));
  const selected = [...launchMatches];
  for (const [category, occurrenceId] of submittedOccurrenceIds) {
    const code = `p158_w6_${category}_${calibrationKey}`;
    const matches = delta.filter((record) => record.runtimeEnvironment === 'development' &&
      record.category === category && record.code === code &&
      record.stage === 'p158_w6_calibration' && record.action === 'journal_calibration');
    if (matches.length === 1 && matches[0].occurrenceId !== occurrenceId) {
      fail('w6_five_surface_correlation_invalid',
        `W6 ${category} occurrence differs from its authenticated append response`);
    }
    selected.push(...matches);
  }
  if (launchMatches.length !== 1 || launchMatches[0].occurrenceId !== quiet.record.occurrenceId ||
      selected.length !== 5 || delta.length !== 5 ||
      new Set(selected.map((record) => record.occurrenceId)).size !== 5 ||
      P158_W6_FAILURE_CATEGORIES.some((category) =>
        selected.filter((record) => record.category === category).length !== 1)) {
    fail('w6_five_surface_correlation_invalid',
      'W6 requires exactly one correlated development record for each named failure surface', {
        selectedCategoryCounts: Object.fromEntries(P158_W6_FAILURE_CATEGORIES.map((category) =>
          [category, selected.filter((record) => record.category === category).length])),
      });
  }
  const completedAt = clock();
  if (Date.parse(completedAt) > Date.parse(validated.boundedWindow.notAfter)) {
    fail('w6_journal_time_window_expired', 'W6 journal calibration completed after its bounded execution window');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-w6-five-surface-journal-calibration.v1',
    planId: 'P158', runId, environmentId: 'E2', runtimeEnvironment: 'development',
    productionAccessAllowed: false, candidateSha256: candidate.candidateSha256,
    installedGenerationIdSha256: sha256(candidate.installedGenerationId),
    dashboardOriginSha256: sha256(validated.origin), runtimeManifestSha256: sha256(manifestResult.body),
    executionWindow: validated.boundedWindow, completedAt,
    authenticatedDashboardSession: true, requestedFailureCount: 5,
    observedFailureCount: 5, categories: [...P158_W6_FAILURE_CATEGORIES],
    records: selected.sort((left, right) =>
      P158_W6_FAILURE_CATEGORIES.indexOf(left.category) - P158_W6_FAILURE_CATEGORIES.indexOf(right.category))
      .map(safeProjection),
    baselineReadbackSha256: sha256(before),
    browserManagerQuietReadbackSha256: sha256(quiet.readback),
    browserManagerQuietStableReadCount: quiet.stableReads,
    browserManagerInductionSha256: sha256(induction), finalReadbackSha256: sha256(after),
    malformedLineSeamReceiptSha256: validated.malformedLineReceipt.receiptSha256,
    liveJournalMalformedLineInjected: false, browserManagerLaunchInvoked: true,
    browserProcessSpawnAttempted: false,
    retryAttempted: false, repairAttempted: false, resultState: 'passed',
    sourceBinding: { sourcePath: P158_W6_JOURNAL_CALIBRATION_SOURCE_PATH, sourceSha256: sourceSha256() },
  };
  const artifact = { ...body, artifactSha256: sha256(body) };
  const encoded = JSON.stringify(artifact);
  if (encoded.includes('://') || /password|bearer|cookie/iu.test(encoded)) {
    fail('w6_journal_artifact_privacy_violation', 'W6 journal calibration artifact contains forbidden material');
  }
  return freeze(artifact);
}

export function p158W6JournalCalibrationSourceBinding() {
  return freeze({
    sourcePath: P158_W6_JOURNAL_CALIBRATION_SOURCE_PATH,
    sourceSha256: sourceSha256(),
  });
}
