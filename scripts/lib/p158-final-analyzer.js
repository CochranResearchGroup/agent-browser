import { createHash } from 'node:crypto';

import { canonicalJson } from './p158-campaign-controller.js';
import { auditDashboardFixture } from './p158-dashboard-oracle.js';
import { auditExternalHandoffSession } from './p158-external-handoff-oracle.js';
import { auditCausalEnvelopes } from './p158-logging-auditor.js';

export const P158_FINAL_DISPOSITIONS = Object.freeze([
  'blocking', 'nonblocking_backlog', 'needs_evidence', 'rejected',
]);

export const P158_OUTCOME_CLASSES = Object.freeze([
  'product', 'infrastructure', 'harness', 'safety_stop', 'inconclusive', 'passed', 'blocked',
]);

export const P158_ARCHITECTURE_BOUNDARIES = Object.freeze([
  { criterionId: 'profile_acquisition_owner', boundary: 'One deep Profile acquisition owner', caseIds: ['A01', 'A02', 'A03', 'A07', 'A08', 'A13', 'X10'] },
  { criterionId: 'cohesive_lease_client', boundary: 'One cohesive protected lease-authority client', caseIds: ['A04', 'A05', 'A06', 'A10', 'A11', 'A12'] },
  { criterionId: 'rust_convergence_owner', boundary: 'One Rust install convergence owner', caseIds: ['X01', 'X02', 'X03', 'X04', 'X05', 'X06', 'X07', 'X08', 'X09'] },
  { criterionId: 'contract_aware_output_renderer', boundary: 'Contract-aware domain output renderers behind one presentation policy', caseIds: ['A15', 'D02', 'D03', 'D04'] },
  { criterionId: 'semantic_contract_oracle', boundary: 'One semantic contract oracle across transports and dashboard projections', caseIds: ['A15', 'D01', 'D02', 'D03', 'D04', 'D05'] },
]);

const RESULT_STATES = Object.freeze([
  'passed', 'reproduced_historical_failure', 'new_product_failure', 'harness_failure',
  'inconclusive', 'skipped_blocked', 'safety_stopped',
]);

const FAILURE_STATES = new Set([
  'reproduced_historical_failure', 'new_product_failure', 'harness_failure',
  'inconclusive', 'skipped_blocked', 'safety_stopped',
]);

const REDACTED = new Set(['[redacted]', '<redacted>', '[excluded]', '<excluded>', '[hashed]', '<hashed>']);

export class P158FinalAnalyzerError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158FinalAnalyzerError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158FinalAnalyzerError(code, message, details);
}

function clone(value) {
  return structuredClone(value);
}

function hashBytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

export function stableP158AnalysisHash(value) {
  return hashBytes(Buffer.from(canonicalJson(value)));
}

function without(value, fields) {
  const excluded = new Set(fields);
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => !excluded.has(field)));
}

function recordDigest(record) {
  return stableP158AnalysisHash(without(record, ['sha256', 'byteCount', 'type']));
}

function artifactBytes(artifact) {
  if (artifact.bytes instanceof Uint8Array) return Buffer.from(artifact.bytes);
  if (typeof artifact.content === 'string') {
    return Buffer.from(artifact.content, artifact.contentEncoding === 'base64' ? 'base64' : 'utf8');
  }
  if (artifact.value !== undefined) return Buffer.from(canonicalJson(artifact.value));
  return null;
}

function normalizeArtifacts(input) {
  const records = Array.isArray(input) ? input : Object.values(input ?? {});
  return records.map((entry) => ({
    ...clone(entry.receipt ?? entry),
    bytes: artifactBytes(entry),
  })).sort((left, right) => String(left.artifactId).localeCompare(String(right.artifactId)));
}

function addFinding(findings, finding) {
  const normalized = {
    code: finding.code,
    category: finding.category,
    disposition: finding.disposition,
    criterion: finding.criterion,
    evidenceIds: [...new Set(finding.evidenceIds ?? [])].sort(),
    consequence: finding.consequence,
    reproducer: finding.reproducer,
    confidence: finding.confidence ?? 'high',
    recommendedOwner: finding.recommendedOwner,
    dependsOnCodes: [...new Set(finding.dependsOnCodes ?? [])].sort(),
  };
  const key = stableP158AnalysisHash(normalized);
  if (!findings.some((entry) => entry.key === key)) findings.push({ key, ...normalized });
}

function integrityFinding(findings, code, criterion, evidenceIds, consequence) {
  addFinding(findings, {
    code,
    category: 'harness',
    disposition: 'blocking',
    criterion,
    evidenceIds,
    consequence,
    reproducer: 'Re-run the provider-free W10 analyzer against the unchanged sealed evidence.',
    recommendedOwner: 'campaign-harness',
  });
}

function verifyLedger(ledgerRecords, manifestSha256, findings) {
  const records = [...ledgerRecords].sort((left, right) => left.sequence - right.sequence);
  let previous = null;
  let priorWall = null;
  let priorMonotonic = null;
  const recordIds = new Set();
  for (const [index, record] of records.entries()) {
    const digest = recordDigest(record);
    if (record.sequence !== index || record.previousRecordSha256 !== previous) {
      integrityFinding(findings, 'ledger_chain_invalid', 'W10.1', [record.recordId],
        'The append-only causal sequence cannot be trusted.');
    }
    if (recordIds.has(record.recordId)) {
      integrityFinding(findings, 'duplicate_ledger_record', 'W10.1', [record.recordId],
        'A ledger record identity was reused.');
    }
    if (record.manifestSha256 !== manifestSha256) {
      integrityFinding(findings, 'ledger_manifest_binding_invalid', 'W10.1', [record.recordId],
        'A result record is not bound to the frozen manifest.');
    }
    const wall = Date.parse(record.wallTime);
    if (!Number.isFinite(wall) || (priorWall !== null && wall < priorWall) ||
        !Number.isInteger(record.monotonicTimeNanoseconds) ||
        (priorMonotonic !== null && record.monotonicTimeNanoseconds < priorMonotonic)) {
      integrityFinding(findings, 'clock_alignment_invalid', 'W10.1', [record.recordId],
        'The evidence timeline contains an invalid or inverted clock observation.');
    }
    if (record.sha256 && record.sha256 !== digest) {
      integrityFinding(findings, 'ledger_record_hash_mismatch', 'W10.1', [record.recordId],
        'A ledger record changed after its digest was computed.');
    }
    recordIds.add(record.recordId);
    previous = digest;
    priorWall = wall;
    priorMonotonic = record.monotonicTimeNanoseconds;
  }
  return { records, headSha256: previous };
}

function verifyArtifacts(artifacts, findings) {
  const seen = new Set();
  const digests = new Set(artifacts.map((artifact) => artifact.sha256).filter(Boolean));
  let validCount = 0;
  for (const artifact of artifacts) {
    if (!artifact.artifactId || seen.has(artifact.artifactId)) {
      integrityFinding(findings, 'artifact_identity_invalid', 'W10.1', [artifact.artifactId ?? 'missing'],
        'Artifact identity is missing or duplicated.');
      continue;
    }
    seen.add(artifact.artifactId);
    if (!artifact.bytes || artifact.sha256 !== hashBytes(artifact.bytes) ||
        artifact.byteCount !== artifact.bytes.byteLength) {
      integrityFinding(findings, 'artifact_hash_mismatch', 'W10.1', [artifact.artifactId],
        'Artifact bytes do not match the sealed receipt.');
      continue;
    }
    if ((artifact.parentArtifactSha256s ?? []).some((digest) => !digests.has(digest))) {
      integrityFinding(findings, 'artifact_parent_missing', 'W10.1', [artifact.artifactId],
        'Artifact ancestry references an unknown earlier digest.');
      continue;
    }
    if (artifact.captureState !== 'complete' && !artifact.captureGap) {
      integrityFinding(findings, 'artifact_capture_gap_unexplained', 'W10.1', [artifact.artifactId],
        'Incomplete capture has no explicit gap record.');
    }
    validCount += 1;
  }
  return { artifactCount: artifacts.length, validArtifactCount: validCount };
}

function terminalResults(manifest, records, findings) {
  const scheduled = new Map((manifest.schedule ?? []).map((attempt) => [attempt.attemptId, attempt]));
  const results = [];
  const seen = new Set();
  for (const record of records.filter((entry) => entry.recordType === 'attempt_terminal')) {
    const attempt = record.payload?.attempt ?? {};
    if (!scheduled.has(attempt.attemptId) || seen.has(attempt.attemptId)) {
      integrityFinding(findings, 'terminal_identity_invalid', 'W10.1', [record.recordId],
        'Attempt terminality is unscheduled or duplicated.');
      continue;
    }
    seen.add(attempt.attemptId);
    results.push({
      recordId: record.recordId,
      wallTime: record.wallTime,
      ...clone(attempt),
      resultState: record.payload.resultState,
      effectState: record.payload.effectState,
      retryDisposition: record.payload.retryDisposition,
      firstFailureSignature: record.payload.firstFailureSignature,
      blockerCode: record.payload.blocker?.code ?? record.payload.blocker?.lostPrerequisite ?? null,
      causalIds: clone(record.payload.causalIds ?? {}),
    });
  }
  const missing = [...scheduled.keys()].filter((attemptId) => !seen.has(attemptId));
  if (missing.length > 0) {
    integrityFinding(findings, 'scheduled_attempt_not_terminal', 'W10.1', missing,
      'W10 cannot begin until every scheduled attempt is terminal.');
  }
  const teardown = records.filter((entry) => entry.recordType === 'scheduled_teardown_terminal');
  if (teardown.length !== 1) {
    integrityFinding(findings, 'teardown_terminality_invalid', 'W10.1', teardown.map((entry) => entry.recordId),
      'Scheduled teardown has not produced exactly one retained terminal result.');
  }
  const seals = records.filter((entry) => entry.recordType === 'evidence_seal');
  if (seals.length !== 1 || records.at(-1)?.recordType !== 'evidence_seal' ||
      seals[0]?.payload?.allScheduledAttemptsTerminal !== true ||
      seals[0]?.payload?.teardownTerminal !== true) {
    integrityFinding(findings, 'evidence_seal_invalid', 'W10.1', seals.map((entry) => entry.recordId),
      'The ledger is not terminal and sealed at the W10 boundary.');
  }
  return results.sort((left, right) => left.attemptId.localeCompare(right.attemptId));
}

function scanPolicyViolations(value, path = [], results = []) {
  if (!value || typeof value !== 'object') return results;
  for (const [field, child] of Object.entries(value)) {
    const next = [...path, field];
    if (/repairAttempted|retryAttempted|garbageCollectionAttempted/.test(field) && child === true) {
      results.push(next.join('.'));
    }
    if (/reactionaryRepairAllowed|opportunisticRetryAllowed|undeclaredEffectsAllowed/.test(field) && child !== false) {
      results.push(next.join('.'));
    }
    scanPolicyViolations(child, next, results);
  }
  return results;
}

function scanForbiddenFields(value, forbidden, path = [], results = []) {
  if (!value || typeof value !== 'object') return results;
  for (const [field, child] of Object.entries(value)) {
    const next = [...path, field];
    if (forbidden.has(field) && child !== null && child !== undefined &&
        !(typeof child === 'string' && REDACTED.has(child.toLowerCase()))) {
      results.push(next.join('.'));
    }
    scanForbiddenFields(child, forbidden, next, results);
  }
  return results;
}

function outcomeClass(result) {
  if (result.resultState === 'passed') return 'passed';
  if (['reproduced_historical_failure', 'new_product_failure'].includes(result.resultState)) return 'product';
  if (result.resultState === 'harness_failure') return 'harness';
  if (result.resultState === 'safety_stopped') return 'safety_stop';
  if (result.resultState === 'skipped_blocked') return 'blocked';
  return 'inconclusive';
}

function buildClusters(results) {
  const clusters = new Map();
  for (const result of results.filter((entry) => FAILURE_STATES.has(entry.resultState))) {
    const signature = result.firstFailureSignature ?? result.blockerCode ?? result.resultState;
    const key = `${outcomeClass(result)}:${signature}`;
    if (!clusters.has(key)) clusters.set(key, {
      outcomeClass: outcomeClass(result), signature, attemptIds: [], caseIds: new Set(), causalIds: new Set(),
    });
    const cluster = clusters.get(key);
    cluster.attemptIds.push(result.attemptId);
    cluster.caseIds.add(result.caseId);
    for (const causalId of Object.values(result.causalIds ?? {})) cluster.causalIds.add(causalId);
  }
  return [...clusters.values()].map((cluster) => ({
    ...cluster,
    attemptIds: cluster.attemptIds.sort(),
    caseIds: [...cluster.caseIds].sort(),
    causalIds: [...cluster.causalIds].sort(),
  })).sort((left, right) => left.outcomeClass.localeCompare(right.outcomeClass) ||
    String(left.signature).localeCompare(String(right.signature)));
}

function buildTimelines(results, records) {
  const byAttempt = new Map(results.map((result) => [result.attemptId, []]));
  for (const record of records) {
    const attemptId = record.payload?.attemptId ?? record.payload?.attempt?.attemptId;
    if (byAttempt.has(attemptId)) byAttempt.get(attemptId).push({
      recordId: record.recordId,
      recordType: record.recordType,
      wallTime: record.wallTime,
      monotonicTimeNanoseconds: record.monotonicTimeNanoseconds,
    });
  }
  return results.filter((result) => FAILURE_STATES.has(result.resultState)).map((result) => {
    const events = byAttempt.get(result.attemptId).sort((a, b) =>
      a.monotonicTimeNanoseconds - b.monotonicTimeNanoseconds);
    const divergence = events.find((entry) => entry.recordType === 'attempt_observation') ?? events.at(-1) ?? null;
    return {
      attemptId: result.attemptId,
      caseId: result.caseId,
      resultState: result.resultState,
      earliestDivergenceRecordId: divergence?.recordId ?? result.recordId,
      events,
    };
  });
}

function dimensionValues(attempt) {
  const values = {};
  const dimensions = [
    ...(attempt.dimensionAssignments ?? []),
    ...(attempt.executionUnit?.dimensionAssignment ? [attempt.executionUnit.dimensionAssignment] : []),
  ];
  for (const dimension of dimensions) values[dimension.dimensionId ?? dimension.id] = dimension.value;
  return values;
}

function crossTabs(results, schedule, candidate) {
  const attempts = new Map((schedule.attempts ?? schedule.schedule ?? []).map(
    (attempt) => [attempt.attemptId, attempt],
  ));
  const axisValue = (axis, attempt) => {
    const dimensions = dimensionValues(attempt);
    const matchingDimensions = Object.entries(dimensions).filter(([id]) => {
      if (axis === 'transport') return id.includes('transport');
      if (axis === 'profile') return id.includes('profile');
      if (axis === 'routeState') return id.includes('route');
      if (axis === 'networkProfile') return id.includes('network');
      if (axis === 'concurrency') return id.includes('concurr') || id.includes('client_count');
      return false;
    }).map(([id, value]) => `${id}=${value}`);
    if (matchingDimensions.length > 0) return matchingDimensions.sort().join(',');
    if (axis === 'seed') return String(attempt.seed ?? 'unspecified');
    if (axis === 'runtimeGeneration') return candidate?.installedGenerationId ?? 'unspecified';
    if (axis === 'timeWindow') return attempt.executionUnit?.plannedOffsetSeconds === null ||
      attempt.executionUnit?.plannedOffsetSeconds === undefined
      ? attempt.caseId ?? 'unspecified'
      : String(attempt.executionUnit.plannedOffsetSeconds);
    return 'unspecified';
  };
  const axes = ['seed', 'concurrency', 'transport', 'profile', 'runtimeGeneration', 'routeState', 'networkProfile', 'timeWindow'];
  return Object.fromEntries(axes.map((axis) => {
    const groups = new Map();
    for (const result of results) {
      const attempt = attempts.get(result.attemptId) ?? result;
      const value = axisValue(axis, attempt);
      if (!groups.has(value)) groups.set(value, { total: 0, reproduced: 0, productFailures: 0 });
      const group = groups.get(value);
      group.total += 1;
      if (result.resultState === 'reproduced_historical_failure') group.reproduced += 1;
      if (outcomeClass(result) === 'product') group.productFailures += 1;
    }
    return [axis, [...groups.entries()].map(([value, counts]) => ({ value, ...counts }))
      .sort((left, right) => left.value.localeCompare(right.value))];
  }));
}

function historicalReproduction(registry, results) {
  const byCase = new Map();
  for (const result of results) {
    if (!byCase.has(result.caseId)) byCase.set(result.caseId, []);
    byCase.get(result.caseId).push(result);
  }
  return (registry.families ?? []).map((family) => {
    const relevant = family.caseIds.flatMap((caseId) => byCase.get(caseId) ?? []);
    const reproduced = relevant.filter((result) => result.resultState === 'reproduced_historical_failure').length;
    const blocked = relevant.filter((result) => result.resultState === 'skipped_blocked').length;
    return {
      familyId: family.id,
      caseIds: [...family.caseIds].sort(),
      terminalAttemptCount: relevant.length,
      reproducedAttemptCount: reproduced,
      blockedAttemptCount: blocked,
      reproductionRate: relevant.length === 0 ? null : reproduced / relevant.length,
    };
  });
}

function summarizeIndependentAudits(input, analyzedAt, findings, results) {
  for (const [index, corpus] of (input.loggingEvidence ?? []).entries()) {
    if (corpus?.schemaVersion !== 'agent-browser.p158-logging-evidence-corpus.v1') continue;
    const body = without(corpus, ['corpusSha256']);
    if (corpus.corpusSha256 !== stableP158AnalysisHash(body) || corpus.runId !== input.runId ||
        corpus.candidateSha256 !== input.manifest?.candidate?.candidateSha256) {
      integrityFinding(findings, 'logging_evidence_corpus_integrity_invalid', 'W10.2',
        [corpus.corpusSha256 ?? `logging-corpus-${index}`],
        'A live logging corpus is not self-hashed and bound to the sealed campaign identity.');
    }
  }
  const logging = (input.loggingEvidence ?? []).map((fixtureSet, index) => auditCausalEnvelopes({
    fixtureSet,
    options: { runId: input.runId, auditId: `${input.runId}:w10:logging:${index}`, auditedAt: analyzedAt },
  }));
  const handoff = (input.externalHandoffSessions ?? []).map((session, index) =>
    auditExternalHandoffSession({
      session,
      options: { auditId: `${input.runId}:w10:handoff:${index}`, auditedAt: analyzedAt },
    }));
  const dashboard = (input.dashboardFixtures ?? []).map((fixture, index) => auditDashboardFixture({
    fixture,
    options: { auditId: `${input.runId}:w10:dashboard:${index}`, auditedAt: analyzedAt },
  }));
  verifyCampaignLoggingBindings(input, results, logging, findings);
  for (const [kind, reports] of Object.entries({ logging, handoff, dashboard })) {
    if (reports.length === 0) {
      addFinding(findings, {
        code: `${kind}_raw_evidence_missing`, category: 'harness', disposition: 'needs_evidence',
        criterion: kind === 'logging' ? 'W10.2' : kind === 'dashboard' ? 'W10.6' : 'W10.1',
        evidenceIds: [], consequence: `The ${kind} summary cannot be independently recomputed.`,
        reproducer: 'Provide the sealed raw normalized evidence and rerun W10.',
        recommendedOwner: 'campaign-harness',
      });
    }
    for (const report of reports.filter((entry) => entry.findings.length > 0)) {
      for (const defect of report.findings) addFinding(findings, {
        code: `${kind}:${defect.code}`,
        category: kind === 'logging' ? 'logging' : kind === 'dashboard' ? 'product' : 'infrastructure',
        disposition: defect.severity === 'needs_evidence' ? 'needs_evidence' : 'blocking',
        criterion: kind === 'logging' ? 'W10.2' : kind === 'dashboard' ? 'W10.6' : 'W10.1',
        evidenceIds: [defect.findingId],
        consequence: `${kind} independent recomputation reported ${defect.code}.`,
        reproducer: `Re-run the ${kind} oracle against the sealed raw input digest ${report.inputSha256}.`,
        recommendedOwner: kind === 'logging' ? 'observability' : kind === 'dashboard' ? 'dashboard' : 'presentation',
      });
    }
  }
  return {
    logging: logging.map((report) => ({
      inputSha256: report.inputSha256,
      passed: report.findings.length === 0,
      findingCount: report.findings.length,
      envelopeCount: report.summary.envelopeCount,
      missingRecordCount: report.summary.missingRecordCount,
      sensitiveValueLeakCount: report.summary.sensitiveValueLeakCount,
      captureGapCount: report.summary.captureGapCount,
    })),
    externalHandoff: handoff.map((report) => ({
      inputSha256: report.inputSha256,
      passed: report.passed,
      findingCount: report.findings.length,
      urlObservationCount: report.summary.urlObservationCount,
      ingressCheckCount: report.summary.ingressCheckCount,
      reconnectCount: report.summary.reconnectCount,
    })),
    dashboard: dashboard.map((report) => ({
      inputSha256: report.inputSha256,
      passed: report.passed,
      findingCount: report.findings.length,
      expectedRailRowCount: report.summary.expectedRailRowCount,
      observedRailRowCount: report.summary.observedRailRowCount,
      missingRailRowCount: report.summary.missingRailRowCount,
      duplicateRailRowCount: report.summary.duplicateRailRowCount,
      staleRailRowCount: report.summary.staleRailRowCount,
      wrongRailRowCount: report.summary.wrongRailRowCount,
      timingDistributions: report.timingDistributions,
      resourceSlopes: { inputSha256: report.inputSha256, ...report.resourceSlopes },
    })),
  };
}

const REQUIRED_LOGGING_SURFACE_ROLES = Object.freeze([
  'ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome',
]);
const REQUIRED_BLOCKED_LOGGING_SURFACE_ROLES = Object.freeze([
  'controller_transition', 'pre_execution_blocker', 'terminal_event',
]);

function addLoggingBindingFinding(findings, {
  code, disposition, evidenceIds, consequence, reproducer,
}) {
  addFinding(findings, {
    code,
    category: 'logging',
    disposition,
    criterion: 'W10.2',
    evidenceIds,
    consequence,
    reproducer,
    recommendedOwner: 'campaign-harness',
  });
}

function verifyCampaignLoggingBindings(input, results, loggingReports, findings) {
  const expectations = Array.isArray(input.loggingExpectations) ? input.loggingExpectations : [];
  const expectedByAttempt = new Map();
  for (const expectation of expectations) {
    if (!expectedByAttempt.has(expectation?.attemptId)) expectedByAttempt.set(expectation?.attemptId, []);
    expectedByAttempt.get(expectation?.attemptId).push(expectation);
  }
  const envelopesByRequest = new Map();
  for (const report of loggingReports) {
    for (const envelope of report.envelopes ?? []) {
      if (!envelopesByRequest.has(envelope.requestId)) envelopesByRequest.set(envelope.requestId, []);
      envelopesByRequest.get(envelope.requestId).push(envelope);
    }
  }
  const resultIds = new Set(results.map((result) => result.attemptId));
  for (const expectation of expectations.filter((entry) => !resultIds.has(entry?.attemptId))) {
    addLoggingBindingFinding(findings, {
      code: 'logging_expectation_unscheduled',
      disposition: 'blocking',
      evidenceIds: [expectation?.attemptId ?? 'missing-attempt-id'],
      consequence: 'A logging expectation is not bound to a terminal scheduled attempt.',
      reproducer: 'Compare the sealed logging expectation index with the terminal attempt ledger.',
    });
  }
  for (const result of results) {
    const attemptExpectations = expectedByAttempt.get(result.attemptId) ?? [];
    if (attemptExpectations.length === 0) {
      addLoggingBindingFinding(findings, {
        code: 'logging_expectation_missing',
        disposition: 'needs_evidence',
        evidenceIds: [result.attemptId],
        consequence: 'The terminal attempt has no sealed declaration of its expected causal logging measurements.',
        reproducer: `Add the frozen logging expectation for ${result.attemptId} and rerun W10.`,
      });
      continue;
    }
    const seenRequestIds = new Set();
    for (const expectation of attemptExpectations) {
      const requestId = expectation?.requestId;
      const expectedRoles = [...new Set(expectation?.expectedSurfaceRoles ?? [])].sort();
      const explicitlyBlocked = result.resultState === 'skipped_blocked' &&
        typeof result.blockerCode === 'string' && result.blockerCode.length > 0;
      const requiredRoles = explicitlyBlocked
        ? [...REQUIRED_BLOCKED_LOGGING_SURFACE_ROLES]
        : [
            ...REQUIRED_LOGGING_SURFACE_ROLES,
            ...(expectation?.incidentExpected === true ? ['incident'] : []),
            ...(expectation?.operatorVisible === true ? ['dashboard_projection'] : []),
          ].sort();
      const exactBlockedRoles = explicitlyBlocked &&
        stableP158AnalysisHash(expectedRoles) === stableP158AnalysisHash([...requiredRoles].sort()) &&
        expectation?.incidentExpected === false && expectation?.operatorVisible === false;
      const invalid = typeof requestId !== 'string' || requestId.length === 0 ||
        seenRequestIds.has(requestId) || result.causalIds?.requestId !== requestId ||
        expectation?.incidentExpected !== (expectedRoles.includes('incident')) ||
        expectation?.operatorVisible !== (expectedRoles.includes('dashboard_projection')) ||
        (explicitlyBlocked ? !exactBlockedRoles : requiredRoles.some((role) => !expectedRoles.includes(role))) ||
        (!explicitlyBlocked && expectedRoles.some((role) =>
          REQUIRED_BLOCKED_LOGGING_SURFACE_ROLES.includes(role) && role !== 'terminal_event'));
      seenRequestIds.add(requestId);
      if (invalid) {
        addLoggingBindingFinding(findings, {
          code: 'logging_expectation_invalid',
          disposition: 'blocking',
          evidenceIds: [result.attemptId, requestId ?? 'missing-request-id'],
          consequence: 'The logging expectation is duplicate, incomplete, or not joined by an immutable terminal causal ID.',
          reproducer: `Validate the sealed logging expectation for ${result.attemptId}.`,
        });
        continue;
      }
      const envelopes = envelopesByRequest.get(requestId) ?? [];
      if (envelopes.length === 0) {
        addLoggingBindingFinding(findings, {
          code: 'logging_attempt_envelope_missing',
          disposition: 'needs_evidence',
          evidenceIds: [result.attemptId, requestId],
          consequence: 'No audited causal envelope matches the terminal attempt request ID.',
          reproducer: `Locate or explicitly record the missing causal measurements for request ${requestId}.`,
        });
        continue;
      }
      if (envelopes.length !== 1) {
        addLoggingBindingFinding(findings, {
          code: 'logging_attempt_envelope_ambiguous',
          disposition: 'blocking',
          evidenceIds: [result.attemptId, requestId, ...envelopes.map((entry) => entry.envelopeId)],
          consequence: 'More than one audited envelope claims the same immutable request ID.',
          reproducer: `Reconcile duplicate envelope ownership for request ${requestId}.`,
        });
        continue;
      }
      const envelope = envelopes[0];
      const auditedExpectedRoles = [...envelope.expectedSurfaceRoles].sort();
      if (stableP158AnalysisHash(expectedRoles) !== stableP158AnalysisHash(auditedExpectedRoles) ||
          envelope.incidentExpected !== expectation.incidentExpected ||
          envelope.operatorVisible !== expectation.operatorVisible) {
        addLoggingBindingFinding(findings, {
          code: 'logging_expected_surface_mismatch',
          disposition: 'blocking',
          evidenceIds: [result.attemptId, requestId, envelope.envelopeId],
          consequence: 'The logging auditor evaluated a weaker or different surface contract than the sealed attempt expectation.',
          reproducer: `Compare expected and audited surface roles for request ${requestId}.`,
        });
      }
    }
  }
}

function correlation(pairs, rightField) {
  if (pairs.length < 2) return null;
  const meanLeft = pairs.reduce((sum, entry) => sum + entry.pressure, 0) / pairs.length;
  const meanRight = pairs.reduce((sum, entry) => sum + entry[rightField], 0) / pairs.length;
  const numerator = pairs.reduce((sum, entry) =>
    sum + (entry.pressure - meanLeft) * (entry[rightField] - meanRight), 0);
  const left = Math.sqrt(pairs.reduce((sum, entry) => sum + (entry.pressure - meanLeft) ** 2, 0));
  const right = Math.sqrt(pairs.reduce((sum, entry) => sum + (entry[rightField] - meanRight) ** 2, 0));
  return left === 0 || right === 0 ? null : numerator / (left * right);
}

function performanceAnalysis(dashboardReports, pressureSamples = []) {
  const timings = dashboardReports.flatMap((report) => report.timingDistributions.map((entry) => ({
    inputSha256: report.inputSha256,
    ...entry,
  })));
  const resourceSlopes = dashboardReports.flatMap((report) => Array.isArray(report.resourceSlopes)
    ? report.resourceSlopes.map((entry) => ({ inputSha256: report.inputSha256, ...entry }))
    : [{ inputSha256: report.inputSha256, ...report.resourceSlopes }]);
  return {
    timings,
    resourceSlopes,
    timeoutDistribution: timings.filter((entry) => entry.budgetMiss),
    pressureCorrelations: [
      { metric: 'latencyMs', coefficient: correlation(pressureSamples, 'latencyMs') },
      { metric: 'failure', coefficient: correlation(pressureSamples.map((entry) => ({
        ...entry, failure: entry.failed ? 1 : 0,
      })), 'failure') },
    ],
  };
}

function assessCriteria(criteria, results) {
  const byCase = new Map();
  for (const result of results) {
    if (!byCase.has(result.caseId)) byCase.set(result.caseId, []);
    byCase.get(result.caseId).push(result);
  }
  return (criteria ?? []).map((criterion) => {
    const relevant = (criterion.caseIds ?? []).flatMap((caseId) => byCase.get(caseId) ?? []);
    const disproven = relevant.some((result) => outcomeClass(result) === 'product');
    const complete = relevant.length > 0 && relevant.every((result) => result.resultState === 'passed');
    return {
      criterionId: criterion.criterionId,
      statement: criterion.statement,
      status: disproven ? 'disproven' : complete ? 'proven' : 'untested',
      evidenceAttemptIds: relevant.map((result) => result.attemptId).sort(),
    };
  });
}

function assessArchitecture(criteria, findings, results) {
  return (criteria ?? []).map((criterion) => {
    const linked = findings.filter((finding) =>
      (criterion.findingCodes ?? []).includes(finding.code) ||
      (criterion.caseIds ?? []).includes(results.find((result) =>
        finding.evidenceIds.includes(result.attemptId))?.caseId));
    return {
      criterionId: criterion.criterionId,
      boundary: criterion.boundary,
      assessment: linked.some((finding) => finding.category === 'product')
        ? 'contradicted'
        : linked.length > 0 ? 'supported' : 'insufficient_evidence',
      findingCodes: linked.map((finding) => finding.code).sort(),
    };
  });
}

function finalizeFindings(findings) {
  return findings.sort((left, right) =>
    left.category.localeCompare(right.category) || left.code.localeCompare(right.code) ||
    left.key.localeCompare(right.key)).map(({ key, ...finding }, index) => ({
    findingId: `p158-w10-finding-${String(index + 1).padStart(5, '0')}-${key.slice(0, 10)}`,
    ...finding,
  }));
}

function remediationGraph(findings) {
  const nodes = findings.filter((finding) => !['rejected'].includes(finding.disposition)).map((finding) => ({
    nodeId: `remediation:${finding.findingId}`,
    findingId: finding.findingId,
    category: finding.category,
    disposition: finding.disposition,
    owner: finding.recommendedOwner,
  }));
  const byCode = new Map(findings.map((finding) => [finding.code, finding]));
  const edges = findings.flatMap((finding) => finding.dependsOnCodes.map((code) => {
    const dependency = byCode.get(code);
    return dependency ? {
      fromNodeId: `remediation:${dependency.findingId}`,
      toNodeId: `remediation:${finding.findingId}`,
      reason: 'explicit_finding_dependency',
    } : null;
  }).filter(Boolean)).sort((left, right) => left.fromNodeId.localeCompare(right.fromNodeId) ||
    left.toNodeId.localeCompare(right.toNodeId));
  return { nodes, edges };
}

export function analyzeP158SealedCampaign({
  sealedCampaign, architectureCriteria = P158_ARCHITECTURE_BOUNDARIES, p157Criteria = [], clock = {},
}) {
  if (!sealedCampaign || typeof sealedCampaign !== 'object') fail('input_missing', 'sealedCampaign is required');
  const input = clone(sealedCampaign);
  if (!input.manifest || !Array.isArray(input.ledgerRecords) || !input.registry) {
    fail('sealed_evidence_incomplete', 'Manifest, ledgerRecords, and historical registry are required');
  }
  const analyzedAt = clock.wallNow?.() ?? input.analyzedAt ?? '1970-01-01T00:00:00.000Z';
  if (!Number.isFinite(Date.parse(analyzedAt))) fail('analysis_time_invalid', 'W10 requires an RFC 3339 analysis time');
  const findings = [];
  const manifestSha256 = input.manifestSha256 ?? stableP158AnalysisHash(input.manifest);
  if (manifestSha256 !== stableP158AnalysisHash(input.manifest)) {
    integrityFinding(findings, 'campaign_manifest_hash_mismatch', 'W10.1', ['campaign-manifest'],
      'The campaign manifest changed after freeze.');
  }
  input.runId = input.runId ?? input.manifest.runId;
  const ledger = verifyLedger(input.ledgerRecords, manifestSha256, findings);
  const artifacts = normalizeArtifacts(input.artifacts);
  const artifactIntegrity = verifyArtifacts(artifacts, findings);
  const results = terminalResults(input.manifest, ledger.records, findings);
  const evidenceSeal = ledger.records.find((record) => record.recordType === 'evidence_seal');
  if (evidenceSeal && (
    evidenceSeal.previousRecordSha256 !== evidenceSeal.payload.ledgerHeadSha256 ||
    evidenceSeal.payload.artifactCount !== artifacts.length ||
    evidenceSeal.artifacts?.length !== 1 ||
    evidenceSeal.artifacts[0].sha256 !== evidenceSeal.payload.manifestSha256
  )) {
    integrityFinding(findings, 'evidence_seal_binding_invalid', 'W10.1', [evidenceSeal.recordId],
      'The evidence seal does not bind its pre-seal ledger head and exact artifact manifest.');
  }
  const policyViolations = scanPolicyViolations({ manifest: input.manifest, records: ledger.records });
  if (policyViolations.length > 0) integrityFinding(findings, 'freeze_policy_violation', 'W10.1', policyViolations,
    'The sealed campaign reports repair, retry, garbage collection, or undeclared effects.');
  const forbidden = new Set(input.registry.forbiddenCapturedFields ?? []);
  const exclusionViolations = scanForbiddenFields({
    manifest: input.manifest,
    ledgerRecords: ledger.records,
    artifactReceipts: artifacts.map(({ bytes: _bytes, content: _content, value: _value, ...receipt }) => receipt),
  }, forbidden);
  if (exclusionViolations.length > 0) addFinding(findings, {
    code: 'forbidden_capture_present', category: 'harness', disposition: 'blocking', criterion: 'W10.1',
    evidenceIds: exclusionViolations, consequence: 'The redacted review boundary was violated.',
    reproducer: 'Run the exclusion scanner against the sealed metadata and raw logging indexes.',
    recommendedOwner: 'evidence-custody',
  });
  const independentAudits = summarizeIndependentAudits(input, analyzedAt, findings, results);
  for (const intervention of ledger.records.filter((record) => record.recordType === 'external_intervention')) {
    addFinding(findings, {
      code: 'external_intervention', category: 'infrastructure', disposition: 'blocking', criterion: 'W10.1',
      evidenceIds: [intervention.recordId], consequence: 'Frozen-state comparison after this intervention is invalid.',
      reproducer: 'Inspect the sealed external intervention record without repeating the effect.',
      recommendedOwner: 'campaign-operations',
    });
  }
  for (const result of results.filter((entry) => FAILURE_STATES.has(entry.resultState))) addFinding(findings, {
    code: `terminal:${result.resultState}:${result.firstFailureSignature ?? result.blockerCode ?? 'unspecified'}`,
    category: outcomeClass(result),
    disposition: outcomeClass(result) === 'product' ? 'blocking' : 'needs_evidence',
    criterion: 'W10.3', evidenceIds: [result.attemptId, result.recordId],
    consequence: `A scheduled attempt terminated as ${result.resultState}.`,
    reproducer: `Replay only the frozen deterministic attempt ${result.attemptId} in successor work.`,
    recommendedOwner: outcomeClass(result) === 'product' ? 'product-runtime' : 'campaign-harness',
  });
  const finalFindings = finalizeFindings(findings);
  const graph = remediationGraph(finalFindings);
  const resultSet = {
    runId: input.runId,
    candidateSha256: input.manifest.candidate?.candidateSha256 ?? input.manifest.candidateSha256,
    manifestSha256,
    ledgerHeadSha256: ledger.headSha256,
    resultCounts: Object.fromEntries(RESULT_STATES.map(
      (state) => [state, results.filter((entry) => entry.resultState === state).length],
    )),
    results,
    clusters: buildClusters(results),
    timelines: buildTimelines(results, ledger.records),
    historicalReproduction: historicalReproduction(input.registry, results),
    reproductionCrossTabs: crossTabs(results, input.executionSchedule ?? input.manifest, input.manifest.candidate),
  };
  const architectureAssessments = assessArchitecture(architectureCriteria, finalFindings, results);
  const p157Acceptance = assessCriteria(p157Criteria, results);
  const reportWithoutHashes = {
    schemaVersion: 'agent-browser.p158-final-analysis.v1',
    planId: 'P158',
    runId: input.runId,
    analyzedAt,
    inputSha256: stableP158AnalysisHash(input),
    repairAttempted: false,
    effectsAttempted: false,
    integrity: {
      passed: !finalFindings.some((finding) => [
        'campaign_manifest_hash_mismatch', 'ledger_chain_invalid', 'duplicate_ledger_record',
        'ledger_manifest_binding_invalid', 'clock_alignment_invalid', 'ledger_record_hash_mismatch',
        'artifact_identity_invalid', 'artifact_hash_mismatch', 'artifact_parent_missing',
        'artifact_capture_gap_unexplained', 'terminal_identity_invalid', 'scheduled_attempt_not_terminal',
        'teardown_terminality_invalid', 'evidence_seal_invalid', 'evidence_seal_binding_invalid',
        'freeze_policy_violation', 'forbidden_capture_present',
      ].includes(finding.code)),
      manifestSha256,
      ledgerHeadSha256: ledger.headSha256,
      ledgerRecordCount: ledger.records.length,
      ...artifactIntegrity,
      policyViolationCount: policyViolations.length,
      exclusionViolationCount: exclusionViolations.length,
    },
    independentAudits,
    performance: performanceAnalysis(independentAudits.dashboard, input.pressureSamples ?? []),
    dashboardReconciliation: independentAudits.dashboard.map((report) => ({
      inputSha256: report.inputSha256,
      expectedRailRowCount: report.expectedRailRowCount,
      observedRailRowCount: report.observedRailRowCount,
      missingRailRowCount: report.missingRailRowCount,
      duplicateRailRowCount: report.duplicateRailRowCount,
      staleRailRowCount: report.staleRailRowCount,
      wrongRailRowCount: report.wrongRailRowCount,
    })),
    resultSet,
    architectureAssessments,
    regressionAssignments: finalFindings.map((finding) => ({
      findingId: finding.findingId,
      seam: finding.category === 'product' ? 'focused_contract_or_integration' :
        finding.category === 'logging' ? 'causal_envelope_contract' : 'provider_free_harness_contract',
    })),
    p157Acceptance,
    findings: finalFindings,
    remediationGraph: graph,
  };
  const resultSetSha256 = stableP158AnalysisHash(resultSet);
  const remediationGraphSha256 = stableP158AnalysisHash(graph);
  return {
    ...reportWithoutHashes,
    resultSetSha256,
    remediationGraphSha256,
    reportSha256: stableP158AnalysisHash({
      ...reportWithoutHashes, resultSetSha256, remediationGraphSha256,
    }),
  };
}
