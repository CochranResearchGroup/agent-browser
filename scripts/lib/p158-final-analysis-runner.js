import { realpath, readFile, readdir } from 'node:fs/promises';
import { dirname, isAbsolute, join, normalize, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, createFileArtifactStore, sha256 } from './p158-campaign-controller.js';
import { analyzeP158SealedCampaign, stableP158AnalysisHash } from './p158-final-analyzer.js';

export const P158_FINAL_ANALYSIS_RUNNER_SOURCE_PATH = 'scripts/lib/p158-final-analysis-runner.js';
export const P158_FINAL_ANALYSIS_PATH = 'analysis/p158-final-analysis.json';
export const P158_FINAL_REVIEW_PATH = 'analysis/p158-redacted-review-candidate.json';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SHA256 = /^[a-f0-9]{64}$/u;
const STRUCTURED_ANALYSIS_ROLES = new Set([
  'logging_evidence', 'dashboard_fixture', 'external_handoff_session', 'pressure_samples',
  'logging_operation_gaps', 'analysis_role_assignments', 'evidence_manifest',
]);
const REDACTED = new Set(['[redacted]', '<redacted>', '[excluded]', '<excluded>', '[hashed]', '<hashed>']);
const REQUIRED_SOURCE_BINDINGS = Object.freeze({
  'p158.final_analysis_descriptor': 'scripts/lib/p158-final-analysis-descriptor.js',
  'p158.final_analysis_runner': P158_FINAL_ANALYSIS_RUNNER_SOURCE_PATH,
});

export class P158FinalAnalysisRunnerError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158FinalAnalysisRunnerError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158FinalAnalysisRunnerError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function pathInside(parent, child) {
  const candidate = relative(resolve(parent), resolve(child));
  return candidate === '' || (!candidate.startsWith(`..${sep}`) && candidate !== '..' && !isAbsolute(candidate));
}

function safeRelativePath(value, field) {
  if (typeof value !== 'string' || value.length === 0 || value.includes('\0') || isAbsolute(value)) {
    fail('analysis_descriptor_path_invalid', `${field} must be a relative path`);
  }
  const normalized = normalize(value);
  if (normalized === '..' || normalized.startsWith(`..${sep}`) || normalized === '.') {
    fail('analysis_descriptor_path_invalid', field);
  }
  return normalized;
}

async function readOptional(store, path) {
  try { return await store.read(path); } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

function parseJson(bytes, field) {
  try { return JSON.parse(bytes.toString('utf8')); } catch {
    fail('sealed_json_invalid', `${field} is not valid JSON`);
  }
}

async function loadBoundFile({ runRoot, realRunRoot, binding, field, json = true }) {
  const relativePath = safeRelativePath(binding?.relativePath, `${field}.relativePath`);
  if (!SHA256.test(binding?.sha256 ?? '') || !Number.isInteger(binding?.byteCount) ||
      binding.byteCount < 0) {
    fail('analysis_descriptor_binding_invalid', `${field} requires sha256 and byteCount`);
  }
  const absolutePath = join(runRoot, relativePath);
  let actualPath;
  let bytes;
  try {
    [actualPath, bytes] = await Promise.all([realpath(absolutePath), readFile(absolutePath)]);
  } catch (error) {
    fail('sealed_artifact_missing', `${field} is missing`, { relativePath, cause: error.message });
  }
  if (!pathInside(realRunRoot, actualPath)) fail('sealed_path_escape', `${field} resolves outside the run root`);
  const actualSha256 = sha256(bytes);
  if (actualSha256 !== binding.sha256 || bytes.byteLength !== binding.byteCount) {
    fail('sealed_artifact_binding_mismatch', `${field} changed after sealing`, {
      relativePath, expectedSha256: binding.sha256, actualSha256,
      expectedByteCount: binding.byteCount, actualByteCount: bytes.byteLength,
    });
  }
  return { binding: clone(binding), bytes, value: json ? parseJson(bytes, field) : null };
}

async function verifySourceBindings(bindings) {
  if (!Array.isArray(bindings) || bindings.length !== Object.keys(REQUIRED_SOURCE_BINDINGS).length) {
    fail('analysis_source_binding_invalid', 'Descriptor and runner source bindings are required');
  }
  for (const [hookId, sourcePath] of Object.entries(REQUIRED_SOURCE_BINDINGS)) {
    const binding = bindings.find((entry) => entry.hookId === hookId);
    if (binding?.sourcePath !== sourcePath || !SHA256.test(binding?.sourceSha256 ?? '') ||
        binding.sourceSha256 !== sha256(await readFile(resolve(REPO_ROOT, sourcePath)))) {
      fail('analysis_source_binding_invalid', `${hookId} source identity changed`);
    }
  }
}

function without(value, fields) {
  const excluded = new Set(fields);
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => !excluded.has(field)));
}

function verifyLedger(ledger, manifestSha256) {
  let previous = null;
  for (const [sequence, entry] of ledger.entries()) {
    const record = entry.value;
    const digest = stableP158AnalysisHash(without(record, ['sha256', 'byteCount', 'type']));
    if (record.sequence !== sequence || record.previousRecordSha256 !== previous ||
        record.manifestSha256 !== manifestSha256 || entry.binding.sha256 !== digest) {
      fail('sealed_ledger_integrity_invalid', `Ledger record ${sequence} is not in the sealed chain`);
    }
    record.sha256 = digest;
    previous = digest;
  }
  return previous;
}

function manifestScheduleProjection(attempt) {
  return {
    scheduleSequence: attempt.scheduleSequence, scheduleId: attempt.scheduleId,
    caseId: attempt.caseId, attemptId: attempt.attemptId, repetition: attempt.repetition,
    seed: attempt.seed, environmentIds: attempt.environmentIds,
    dependsOnAttemptIds: attempt.dependsOnAttemptIds, preconditionIds: attempt.preconditionIds,
    stimuli: attempt.stimuli, evidenceProfile: attempt.evidenceProfile,
    externalIngressRequired: attempt.externalIngressRequired,
    preExecutionBlocker: attempt.preExecutionBlocker,
  };
}

function assertTerminalBoundary({ manifest, freeze, schedule, ledger, evidenceManifest, evidenceBinding }) {
  const freezeTransition = ledger.find((entry) => entry.value.recordType === 'controller_transition' &&
    entry.value.payload?.to === 'frozen');
  if (manifest?.schemaVersion !== 'agent-browser.p158-campaign-manifest.v1' ||
      freeze?.schemaVersion !== 'agent-browser.p158-campaign-freeze.v1' ||
      freeze.controllerState !== 'frozen' || manifest.runId !== freeze.runId ||
      schedule?.schemaVersion !== 'agent-browser.p158-execution-schedule.v1' ||
      (schedule.runId && schedule.runId !== manifest.runId) ||
      freeze.manifestSha256 !== sha256(canonicalJson(manifest)) ||
      freeze.candidateSha256 !== manifest.candidate?.candidateSha256 ||
      freeze.artifactBindingsSha256 !== sha256(manifest.artifactBindings) ||
      freeze.environmentSealsSha256 !== sha256(manifest.environmentSeals) ||
      freeze.calibrationSha256 !== sha256(manifest.calibration) ||
      freeze.fixtureSealSha256 !== sha256(manifest.fixtureSeal) ||
      freeze.startedCaseCount !== 0 || freeze.startedAttemptCount !== 0 ||
      !freezeTransition || freezeTransition.value.previousRecordSha256 !== freeze.preparedLedgerHeadSha256) {
    fail('sealed_campaign_identity_invalid', 'Manifest, freeze, and schedule do not describe one campaign');
  }
  const seals = ledger.filter((entry) => entry.value.recordType === 'evidence_seal');
  const seal = seals[0]?.value;
  if (seals.length !== 1 || ledger.at(-1)?.value.recordType !== 'evidence_seal' ||
      seal.controllerState !== 'evidence_sealed' || seal.payload?.allScheduledAttemptsTerminal !== true ||
      seal.payload?.teardownTerminal !== true || seal.payload?.manifestSha256 !== evidenceBinding.sha256) {
    fail('campaign_not_evidence_sealed', 'W10 requires the unique terminal evidence seal');
  }
  const expectedAttemptIds = new Set((manifest.schedule ?? []).map((entry) => entry.attemptId));
  const terminals = ledger.filter((entry) => entry.value.recordType === 'attempt_terminal')
    .map((entry) => entry.value.payload?.attempt?.attemptId);
  if (terminals.length !== expectedAttemptIds.size || new Set(terminals).size !== expectedAttemptIds.size ||
      terminals.some((attemptId) => !expectedAttemptIds.has(attemptId)) ||
      ledger.filter((entry) => entry.value.recordType === 'scheduled_teardown_terminal').length !== 1 ||
      evidenceManifest?.schemaVersion !== 'agent-browser.p158-evidence-manifest.v1' ||
      evidenceManifest.runId !== manifest.runId) {
    fail('campaign_attempts_not_terminal', 'Every exact scheduled attempt and teardown must be terminal');
  }
}

function scanForbidden(value, forbidden, path = [], findings = []) {
  if (!value || typeof value !== 'object') return findings;
  for (const [field, child] of Object.entries(value)) {
    const next = [...path, field];
    if (forbidden.has(field) && child !== null && child !== undefined &&
        !(typeof child === 'string' && REDACTED.has(child.toLowerCase()))) {
      findings.push({ field, path: next.join('.') });
    }
    scanForbidden(child, forbidden, next, findings);
  }
  return findings;
}

function artifactReceipt(binding) {
  return {
    artifactId: binding.artifactId,
    relativePath: binding.relativePath,
    mediaType: binding.mediaType ?? 'application/json',
    sha256: binding.sha256,
    byteCount: binding.byteCount,
    captureState: binding.captureState ?? 'complete',
    captureGap: binding.captureGap ?? null,
    redactions: clone(binding.redactions ?? []),
    parentArtifactSha256s: clone(binding.parentArtifactSha256s ?? []),
  };
}

function forbiddenMarkers(violations) {
  const markers = {};
  for (const violation of violations) {
    markers[violation.field] = { present: true, pathSha256: sha256(violation.path) };
  }
  return markers;
}

function exactArtifactSet(evidenceManifest, seal, artifacts) {
  const sealedReceipts = [...(evidenceManifest.artifacts ?? []), ...(seal.artifacts ?? [])];
  const expected = new Map(sealedReceipts.map((entry) => [entry.relativePath, entry]));
  if (expected.size !== artifacts.length || artifacts.some((entry) => {
    const receipt = expected.get(entry.binding.relativePath);
    return !receipt || receipt.artifactId !== entry.binding.artifactId ||
      receipt.sha256 !== entry.binding.sha256 || receipt.byteCount !== entry.binding.byteCount;
  })) fail('sealed_artifact_inventory_mismatch', 'Descriptor artifacts differ from the evidence seal inventory');
}

function roleValues(artifacts, role) {
  return artifacts.filter((entry) => entry.binding.analysisRole === role).flatMap((entry) => {
    if (role === 'logging_evidence') return [entry.value];
    if (role === 'dashboard_fixture') return entry.value.fixtures ?? [entry.value];
    if (role === 'external_handoff_session') return entry.value.sessions ?? [entry.value];
    if (role === 'pressure_samples') return entry.value.samples ?? entry.value;
    return [];
  });
}

function validateLoggingOperationGaps(artifacts, runId) {
  const matching = artifacts.filter((entry) => entry.binding.analysisRole === 'logging_operation_gaps');
  if (matching.length !== 1) {
    fail('logging_operation_gaps_missing', 'Exactly one sealed logging operation-gap artifact is required');
  }
  const value = matching[0].value;
  if (value?.schemaVersion !== 'agent-browser.p158-logging-operation-gaps.v1' ||
      value.planId !== 'P158' || value.runId !== runId || !Array.isArray(value.operations) ||
      value.operationGapCount !== value.operations.length ||
      value.loggingOperationGapsSha256 !== sha256(value.operations) ||
      value.operations.some((gap) => !['A08', 'A13'].includes(gap.caseId) || gap.phaseId !== 'W7' ||
        gap.productRequestId !== null || gap.correlationState !== 'product_request_id_unavailable' ||
        gap.loggingGap?.code !== 'product_request_id_not_preserved')) {
    fail('logging_operation_gaps_invalid', 'The sealed operation-gap artifact is not canonical');
  }
  return clone(value.operations);
}

function curatedReview(report) {
  const body = {
    schemaVersion: 'agent-browser.p158-redacted-review-candidate.v1',
    planId: report.planId,
    runId: report.runId,
    analyzedAt: report.analyzedAt,
    sourceReportSha256: report.reportSha256,
    integrity: clone(report.integrity),
    resultCounts: clone(report.resultSet.resultCounts),
    historicalReproduction: clone(report.resultSet.historicalReproduction),
    architectureAssessments: clone(report.architectureAssessments),
    p157Acceptance: clone(report.p157Acceptance),
    findings: report.findings.map((finding) => ({
      findingId: finding.findingId, code: finding.code, category: finding.category,
      disposition: finding.disposition, criterion: finding.criterion,
      consequence: finding.consequence, confidence: finding.confidence,
      recommendedOwner: finding.recommendedOwner, dependsOnCodes: clone(finding.dependsOnCodes),
    })),
    redaction: {
      mode: 'curated_allowlist', rawArtifactsIncluded: false, rawCausalRecordsIncluded: false,
      browserContentIncluded: false, automaticallyCommitted: false,
    },
  };
  return { ...body, reviewSha256: stableP158AnalysisHash(body) };
}

function verifyExistingAnalysis(value, inputSha256) {
  if (value?.inputSha256 !== inputSha256 ||
      value.reportSha256 !== stableP158AnalysisHash(without(value, ['reportSha256']))) {
    fail('existing_analysis_changed', 'Existing append-only analysis does not match the sealed input');
  }
}

function analysisTerminalRecord({ manifest, ledger, report, review }) {
  const sequence = ledger.length;
  const prior = ledger.at(-1).value;
  return {
    schemaVersion: 'agent-browser.p158-campaign-result.v1', planId: 'P158', runId: manifest.runId,
    manifestSha256: sha256(canonicalJson(manifest)),
    recordId: `${manifest.runId}:record:${String(sequence).padStart(8, '0')}`,
    sequence, previousRecordSha256: stableP158AnalysisHash(without(prior,
      ['sha256', 'byteCount', 'type'])),
    recordType: 'analysis_terminal', controllerState: 'analyzed', wallTime: report.analyzedAt,
    monotonicTimeNanoseconds: prior.monotonicTimeNanoseconds + 1,
    clockOffsetMilliseconds: prior.clockOffsetMilliseconds,
    payload: { kind: 'analysis_terminal', sealedLedgerHeadSha256: stableP158AnalysisHash(without(prior,
      ['sha256', 'byteCount', 'type'])), resultSetSha256: report.resultSetSha256,
    remediationGraphSha256: report.remediationGraphSha256, reportSha256: report.reportSha256,
    reviewSha256: review.reviewSha256, analyzedAt: report.analyzedAt, terminal: true }, artifacts: [],
  };
}

function verifyAnalysisTerminal(value, expected) {
  if (stableP158AnalysisHash(value) !== stableP158AnalysisHash(expected)) {
    fail('existing_analysis_terminal_changed', 'Existing analyzed transition does not bind the report and review');
  }
}

/**
 * Reads one sealed Plan 0158 campaign and writes only append-once analysis artifacts under that run root.
 * It never invokes campaign, browser, provider, route, profile, or runtime operations.
 */
export async function runP158FinalAnalysis({ descriptorPath, descriptorSha256,
  clock = { wallNow: () => new Date().toISOString() } }) {
  if (!isAbsolute(descriptorPath ?? '') || !SHA256.test(descriptorSha256 ?? '')) {
    fail('analysis_descriptor_invalid', 'An absolute descriptor path and exact SHA-256 are required');
  }
  const descriptorBytes = await readFile(descriptorPath);
  if (sha256(descriptorBytes) !== descriptorSha256) fail('analysis_descriptor_changed', 'Descriptor digest mismatch');
  const descriptor = parseJson(descriptorBytes, 'descriptor');
  if (descriptor?.schemaVersion !== 'agent-browser.p158-final-analysis-runner.v1' ||
      descriptor.planId !== 'P158' || !isAbsolute(descriptor.runRoot ?? '') ||
      !pathInside(descriptor.runRoot, descriptorPath) || pathInside(REPO_ROOT, descriptor.runRoot) ||
      !Array.isArray(descriptor.files?.ledger) || !Array.isArray(descriptor.files?.artifacts) ||
      descriptor.output?.analysis !== P158_FINAL_ANALYSIS_PATH ||
      descriptor.output?.reviewCandidate !== P158_FINAL_REVIEW_PATH) {
    fail('analysis_descriptor_invalid', 'Descriptor must bind an external campaign root and fixed outputs');
  }
  const realRunRoot = await realpath(descriptor.runRoot);
  if (!pathInside(realRunRoot, await realpath(descriptorPath))) {
    fail('analysis_descriptor_invalid', 'Descriptor resolves outside the campaign root');
  }
  await verifySourceBindings(descriptor.sourceBindings);
  const store = createFileArtifactStore(descriptor.runRoot);
  const required = ['manifest', 'freeze', 'schedule', 'registry', 'evidenceManifest'];
  const loaded = Object.fromEntries(await Promise.all(required.map(async (field) => [field,
    await loadBoundFile({ runRoot: descriptor.runRoot, realRunRoot, binding: descriptor.files?.[field], field })])));
  const ledger = await Promise.all((descriptor.files?.ledger ?? []).map((binding, index) =>
    loadBoundFile({ runRoot: descriptor.runRoot, realRunRoot, binding, field: `ledger[${index}]` })));
  if (ledger.length === 0) fail('sealed_ledger_missing', 'The sealed ledger is required');
  const manifestSha256 = loaded.manifest.binding.sha256;
  if (loaded.manifest.value.registrySha256 !== sha256(loaded.registry.value) ||
      loaded.schedule.value.registrySha256 !== loaded.manifest.value.registrySha256 ||
      loaded.schedule.value.scheduleSha256 !== sha256(without(loaded.schedule.value,
        ['scheduleSha256', 'adapterReadiness'])) ||
      sha256(loaded.schedule.value.attempts.map(manifestScheduleProjection)) !==
        sha256(loaded.manifest.value.schedule)) {
    fail('sealed_schedule_binding_invalid', 'Registry, schedule, and manifest projections differ');
  }
  const ledgerHeadSha256 = verifyLedger(ledger, manifestSha256);
  if (loaded.freeze.value.manifestSha256 !== manifestSha256) {
    fail('sealed_campaign_identity_invalid', 'Freeze or ledger boundary is inconsistent');
  }
  assertTerminalBoundary({ manifest: loaded.manifest.value, freeze: loaded.freeze.value,
    schedule: loaded.schedule.value, ledger, evidenceManifest: loaded.evidenceManifest.value,
    evidenceBinding: loaded.evidenceManifest.binding });
  const artifacts = await Promise.all((descriptor.files?.artifacts ?? []).map((binding, index) =>
    loadBoundFile({ runRoot: descriptor.runRoot, realRunRoot, binding, field: `artifacts[${index}]`,
      json: false })));
  for (const [index, entry] of artifacts.entries()) {
    if (STRUCTURED_ANALYSIS_ROLES.has(entry.binding.analysisRole) &&
        entry.binding.mediaType !== 'application/json') {
      fail('structured_analysis_artifact_media_type_invalid',
        `${entry.binding.analysisRole} must be declared as application/json`);
    }
    if (entry.binding.mediaType === 'application/json') {
      entry.value = parseJson(entry.bytes, `artifacts[${index}]`);
    }
  }
  for (const requiredRole of ['logging_evidence', 'dashboard_fixture']) {
    if (!artifacts.some((entry) => entry.binding.analysisRole === requiredRole)) {
      fail('sealed_analysis_artifact_missing', `The sealed campaign lacks ${requiredRole}`);
    }
  }
  const seal = ledger.at(-1).value;
  exactArtifactSet(loaded.evidenceManifest.value, seal, artifacts);
  const forbidden = new Set(loaded.registry.value.forbiddenCapturedFields ?? []);
  const artifactInputs = artifacts.map((entry) => {
    const violations = entry.value === null ? [] :
      scanForbidden(entry.value, forbidden, [entry.binding.relativePath]);
    return { ...artifactReceipt(entry.binding), ...forbiddenMarkers(violations), bytes: entry.bytes };
  });
  const sealedCampaign = {
    runId: loaded.manifest.value.runId,
    manifest: loaded.manifest.value,
    manifestSha256,
    freeze: loaded.freeze.value,
    executionSchedule: loaded.schedule.value,
    ledgerRecords: ledger.map((entry) => entry.value),
    artifacts: artifactInputs,
    registry: loaded.registry.value,
    loggingEvidence: roleValues(artifacts, 'logging_evidence'),
    dashboardFixtures: roleValues(artifacts, 'dashboard_fixture'),
    externalHandoffSessions: roleValues(artifacts, 'external_handoff_session'),
    pressureSamples: roleValues(artifacts, 'pressure_samples'),
    loggingOperationGaps: validateLoggingOperationGaps(artifacts, loaded.manifest.value.runId),
    loggingExpectations: clone(descriptor.loggingExpectations ?? []),
  };
  const inputSha256 = stableP158AnalysisHash(sealedCampaign);
  const analysisTerminalPath = `ledger/${String(ledger.length).padStart(8, '0')}-analysis_terminal.json`;
  const analysisTerminalNames = (await readdir(join(descriptor.runRoot, 'ledger')))
    .filter((name) => name.endsWith('-analysis_terminal.json'));
  if (analysisTerminalNames.length > 1 ||
      (analysisTerminalNames.length === 1 && `ledger/${analysisTerminalNames[0]}` !== analysisTerminalPath)) {
    fail('analysis_terminal_duplicate', 'Exactly one canonical analyzed transition is permitted');
  }
  const priorAnalysisBytes = await readOptional(store, P158_FINAL_ANALYSIS_PATH);
  const priorReviewBytes = await readOptional(store, P158_FINAL_REVIEW_PATH);
  const priorTerminalBytes = await readOptional(store, analysisTerminalPath);
  if ((priorReviewBytes || priorTerminalBytes) && !priorAnalysisBytes) {
    fail('existing_analysis_incomplete', 'Review or analyzed transition exists without analysis');
  }
  let report;
  let resumed = false;
  if (priorAnalysisBytes) {
    report = parseJson(priorAnalysisBytes, 'existing analysis');
    verifyExistingAnalysis(report, inputSha256);
    resumed = true;
  } else {
    report = analyzeP158SealedCampaign({ sealedCampaign,
      architectureCriteria: descriptor.architectureCriteria,
      p157Criteria: descriptor.p157Criteria,
      clock });
    await store.writeOnce(P158_FINAL_ANALYSIS_PATH, canonicalJson(report));
  }
  let review;
  if (priorReviewBytes) {
    review = parseJson(priorReviewBytes, 'existing review');
    const reviewBody = without(review, ['reviewSha256']);
    if (review.reviewSha256 !== stableP158AnalysisHash(reviewBody) ||
        review.sourceReportSha256 !== report.reportSha256) {
      fail('existing_review_changed', 'Existing append-only review candidate changed');
    }
  } else {
    review = curatedReview(report);
    await store.writeOnce(P158_FINAL_REVIEW_PATH, canonicalJson(review));
  }
  const terminal = analysisTerminalRecord({ manifest: loaded.manifest.value, ledger, report, review });
  if (priorTerminalBytes) verifyAnalysisTerminal(parseJson(priorTerminalBytes,
    'existing analysis terminal'), terminal);
  else await store.writeOnce(analysisTerminalPath, canonicalJson(terminal));
  return clone({ report, reviewCandidate: review, resumed, effectsAttempted: false,
    repairAttempted: false, controllerState: 'analyzed', analysisTerminalSha256: sha256(canonicalJson(terminal)),
    outputPaths: [P158_FINAL_ANALYSIS_PATH, P158_FINAL_REVIEW_PATH, analysisTerminalPath] });
}
