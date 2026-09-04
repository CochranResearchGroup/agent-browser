#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { promises as dns } from 'node:dns';
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { connect as tlsConnect } from 'node:tls';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import {
  EXTERNAL_HANDOFF_FINDING_CODES,
  EXTERNAL_URL_ROLES,
  auditExternalHandoffSession,
  classifyOperatorUrl,
} from './lib/p158-external-handoff-oracle.js';
import { sha256 as campaignSha256 } from './lib/p158-campaign-controller.js';

export const EXTERNAL_VANTAGE_RECEIPT_SCHEMA =
  'agent-browser.p158-external-vantage-receipt.v1';
export const EXTERNAL_CALIBRATION_RECEIPT_SCHEMA =
  'agent-browser.p158-external-calibration-receipt.v1';
export const EXTERNAL_VANTAGE_AGGREGATE_SCHEMA =
  'agent-browser.p158-external-vantage-aggregate.v1';
export const W8_EXTERNAL_ACTION_RESULT_SCHEMA =
  'agent-browser.p158-w8-external-action-result.v1';
export const PINNED_PLAYWRIGHT_VERSION = '1.62.1';
export const CALIBRATION_DURATION_MS = 20 * 60 * 1000;
export const CALIBRATION_LATE_TOLERANCE_MS = 30 * 1000;
export const CALIBRATION_MINIMUM_LEAD_MS = 2 * 60 * 1000;
const REQUIRED_INGRESS_KINDS = [
  'dns',
  'tls',
  'redirect',
  'cookie',
  'websocket',
  'iframe',
  'form_action',
  'reconnect',
];

const INTERNAL_HOST = /(^|\.)(localhost|[^.]+\.local)$|^(127\.|10\.|192\.168\.|169\.254\.)|^172\.(1[6-9]|2\d|3[01])\./i;
const REQUIRED_IDENTITY_FIELDS = [
  'browserId',
  'profileId',
  'sessionId',
  'tabId',
  'targetId',
  'visibleUrl',
  'pageMarker',
  'pixelHash',
];

export function canonicalHash(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

export function validateExternalVantageConfiguration({ env, clientId, paceProfile, mode = 'readiness' }) {
  if (!['human_controller', 'slow_concurrency'].includes(paceProfile)) {
    throw new Error(`Unsupported pace profile: ${paceProfile}`);
  }
  if (!['readiness', 'calibration'].includes(mode)) throw new Error(`Unsupported probe mode: ${mode}`);
  if (!/^external-runner-[a-z0-9-]+$/.test(clientId || '')) {
    throw new Error('External vantage client ID must be an immutable external-runner ID');
  }
  if (env.P158_W8_ACTION_MANIFEST_SHA256 &&
      !/^[a-f0-9]{64}$/.test(env.P158_W8_ACTION_MANIFEST_SHA256)) {
    throw new Error('W8 action manifest binding must be a lowercase SHA-256 digest');
  }
  for (const name of [
    'P158_DEV_HANDOFF_URL',
    'P158_DEV_DASHBOARD_USERNAME',
    'P158_DEV_DASHBOARD_PASSWORD',
    'P158_DEV_EXPECTED_IDENTITY_JSON',
    'P158_DEV_PIXEL_MARKER_REGION_JSON',
    'P158_DEV_VISUAL_FIXTURE_ATTESTATION_JSON',
    'P158_RUN_ID',
  ]) {
    if (!env[name]?.trim()) throw new Error(`Missing required secret or binding: ${name}`);
  }
  if (
    env.GITHUB_ACTIONS !== 'true' ||
    env.RUNNER_ENVIRONMENT !== 'github-hosted' ||
    !env.GITHUB_RUN_ID ||
    !env.GITHUB_JOB ||
    !env.RUNNER_NAME
  ) {
    throw new Error('External vantage must run on an attributable GitHub-hosted runner');
  }
  const handoff = new URL(env.P158_DEV_HANDOFF_URL);
  const classification = classifyOperatorUrl(handoff.href, { role: 'starting_handoff' });
  if (!classification.valid || classification.findingCodes.length > 0) {
    throw new Error('Handoff secret is not a public HTTPS durable remote-view URL');
  }
  const expectedIdentity = parseSecretJson(env.P158_DEV_EXPECTED_IDENTITY_JSON, 'expected identity');
  for (const field of REQUIRED_IDENTITY_FIELDS) {
    if (typeof expectedIdentity[field] !== 'string' || !expectedIdentity[field].trim()) {
      throw new Error(`Expected identity is missing ${field}`);
    }
  }
  if (!/^[a-f0-9]{64}$/.test(expectedIdentity.pixelHash)) {
    throw new Error('Expected identity pixelHash must be a lowercase SHA-256 digest');
  }
  const pixelMarkerRegion = parseSecretJson(
    env.P158_DEV_PIXEL_MARKER_REGION_JSON,
    'pixel marker region',
  );
  for (const field of ['x', 'y', 'width', 'height']) {
    if (!Number.isInteger(pixelMarkerRegion[field]) || pixelMarkerRegion[field] < 0) {
      throw new Error(`Pixel marker region has invalid ${field}`);
    }
  }
  if (pixelMarkerRegion.width < 1 || pixelMarkerRegion.height < 1) {
    throw new Error('Pixel marker region must have positive dimensions');
  }
  if (!['viewport', 'remote-view-iframe'].includes(pixelMarkerRegion.coordinateSpace || 'viewport')) {
    throw new Error('Pixel marker region has invalid coordinateSpace');
  }
  if (
    pixelMarkerRegion.x + pixelMarkerRegion.width > 1440 ||
    pixelMarkerRegion.y + pixelMarkerRegion.height > 1000
  ) {
    throw new Error('Pixel marker region must fit the frozen 1440 by 1000 viewport');
  }
  const visualFixtureAttestation = parseSecretJson(
    env.P158_DEV_VISUAL_FIXTURE_ATTESTATION_JSON,
    'visual fixture attestation',
  );
  if (
    visualFixtureAttestation.syntheticOnly !== true ||
    visualFixtureAttestation.forbiddenPrivateFieldsExcluded !== true ||
    typeof visualFixtureAttestation.fixtureId !== 'string' ||
    !/^[a-f0-9]{64}$/.test(visualFixtureAttestation.redactionReceiptSha256 || '')
  ) {
    throw new Error('Visual fixture attestation does not prove the synthetic redaction boundary');
  }
  let calibrationDescriptor = null;
  if (mode === 'calibration') {
    if (!env.P158_CALIBRATION_START_AT?.trim()) {
      throw new Error('Missing required secret or binding: P158_CALIBRATION_START_AT');
    }
    if (!/^[a-f0-9]{40}$/.test(env.P158_CANDIDATE_COMMIT || '')) {
      throw new Error('Calibration candidate commit must be an exact full SHA');
    }
    calibrationDescriptor = buildExternalCalibrationDescriptor({
      runId: env.P158_RUN_ID,
      candidateCommit: env.P158_CANDIDATE_COMMIT,
      workflowRunId: env.GITHUB_RUN_ID,
      workflowRunAttempt: Number(env.GITHUB_RUN_ATTEMPT),
      handoffUrlSha256: hashText(handoff.href),
      calibrationStartAt: env.P158_CALIBRATION_START_AT,
    });
  }
  return {
    handoff,
    expectedIdentity,
    pixelMarkerRegion,
    visualFixtureAttestation,
    calibrationDescriptor,
  };
}

export function buildExternalCalibrationSchedule({
  durationMs = CALIBRATION_DURATION_MS,
  actionCount = 25,
  reconnectCount = 5,
} = {}) {
  if (durationMs < CALIBRATION_DURATION_MS || actionCount !== 25 || reconnectCount !== 5) {
    throw new Error('C01 external calibration requires at least 20 minutes, 25 actions, and 5 reconnects per client');
  }
  const events = [];
  for (let ordinal = 1; ordinal <= actionCount; ordinal += 1) {
    const offsetMs = Math.floor((ordinal * durationMs) / (actionCount + 1));
    events.push({ kind: 'dashboard_action', ordinal, offsetMs });
    if (ordinal % (actionCount / reconnectCount) === 0) {
      events.push({ kind: 'handoff_reconnect', ordinal: ordinal / (actionCount / reconnectCount), offsetMs });
    }
  }
  return events;
}

export function buildExternalCalibrationDescriptor({
  runId,
  candidateCommit,
  workflowRunId,
  workflowRunAttempt,
  handoffUrlSha256,
  calibrationStartAt,
}) {
  if (typeof runId !== 'string' || !runId.trim()) throw new Error('Calibration run ID is required');
  if (!/^[a-f0-9]{40}$/.test(candidateCommit || '')) {
    throw new Error('Calibration candidate commit must be an exact full SHA');
  }
  if (!/^\d+$/.test(workflowRunId || '') || !Number.isInteger(workflowRunAttempt) || workflowRunAttempt < 1) {
    throw new Error('Calibration workflow run ID and attempt are required');
  }
  if (!/^[a-f0-9]{64}$/.test(handoffUrlSha256 || '')) {
    throw new Error('Calibration durable handoff digest is required');
  }
  if (
    typeof calibrationStartAt !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?Z$/.test(calibrationStartAt)
  ) {
    throw new Error('Calibration start must be an RFC3339 UTC timestamp');
  }
  const startMs = Date.parse(calibrationStartAt);
  if (!Number.isFinite(startMs)) {
    throw new Error('Calibration start must be a canonical RFC3339 UTC timestamp');
  }
  const normalizedStartAt = new Date(startMs).toISOString();
  const schedule = buildExternalCalibrationSchedule();
  const descriptor = {
    schemaVersion: 'agent-browser.p158-external-calibration-dispatch.v1',
    planId: 'P158',
    runId,
    candidateCommit,
    workflowRunId,
    workflowRunAttempt,
    handoffUrlSha256,
    calibrationStartAt: normalizedStartAt,
    calibrationEndAt: new Date(startMs + CALIBRATION_DURATION_MS).toISOString(),
    durationMs: CALIBRATION_DURATION_MS,
    lateToleranceMs: CALIBRATION_LATE_TOLERANCE_MS,
    actionCountPerClient: 25,
    reconnectCountPerClient: 5,
    scheduleSha256: canonicalHash(schedule),
  };
  return { ...descriptor, descriptorSha256: canonicalHash(descriptor) };
}

export function validateExternalCalibrationLeadTime(
  descriptor,
  nowMs = Date.now(),
  minimumLeadMs = CALIBRATION_MINIMUM_LEAD_MS,
) {
  const startMs = Date.parse(descriptor?.calibrationStartAt);
  if (!Number.isFinite(startMs) || startMs - nowMs < minimumLeadMs) {
    throw new Error(`Calibration start must remain at least ${minimumLeadMs}ms in the future when the probe begins`);
  }
  return { observedAt: new Date(nowMs).toISOString(), leadTimeMs: startMs - nowMs };
}

export function redactOperatorUrl(value) {
  try {
    const url = new URL(value);
    const segments = url.pathname.split('/').filter(Boolean);
    if (segments[0] === 'remote-view' && segments[1]) {
      url.pathname = '/remote-view/<redacted>';
    }
    url.username = '';
    url.password = '';
    url.search = '';
    url.hash = '';
    return url.href;
  } catch {
    return '<invalid-or-redacted-url>';
  }
}

export function findInternalUrlLeaks(urlEvidence) {
  const findings = [];
  for (const evidence of urlEvidence) {
    const classified = classifyOperatorUrl(evidence.url, {
      role: evidence.role,
      baseUrl: evidence.baseUrl,
      resolvedAddresses: evidence.resolvedAddresses ?? [],
    });
    if (classified.findingCodes.length > 0) {
      findings.push({
        evidenceId: evidence.evidenceId,
        role: evidence.role,
        findingCodes: classified.findingCodes,
      });
    }
  }
  return findings;
}

export function aggregateExternalVantageReceipts(receipts, { runId }) {
  if (!Array.isArray(receipts) || receipts.length !== 2) {
    throw new Error('External vantage aggregation requires exactly two receipts');
  }
  const ordered = structuredClone(receipts).sort((a, b) => a.clientId.localeCompare(b.clientId));
  for (const receipt of ordered) {
    const expectedSchema = receipt.mode === 'calibration'
      ? EXTERNAL_CALIBRATION_RECEIPT_SCHEMA
      : EXTERNAL_VANTAGE_RECEIPT_SCHEMA;
    if (receipt.schemaVersion !== expectedSchema || receipt.success !== true) {
      throw new Error(`External vantage receipt is not successful: ${receipt.clientId || 'unknown'}`);
    }
    if (receipt.planId !== 'P158') throw new Error('External vantage receipt plan binding mismatch');
    if (!['readiness', 'calibration'].includes(receipt.mode)) {
      throw new Error('External vantage receipt has invalid probe mode');
    }
    if (receipt.runId !== runId) throw new Error('External vantage receipt run binding mismatch');
    if (receipt.repairAttempted !== false || receipt.retryCount !== 0) {
      throw new Error('External vantage receipt records repair or retry');
    }
    if (receipt.mode === 'calibration') {
      const { receiptSha256, ...body } = receipt;
      if (receiptSha256 !== campaignSha256(body)) {
        throw new Error('External calibration receipt self-hash mismatch');
      }
      if (
        receipt.sourceCommit !== receipt.calibration.dispatchDescriptor.candidateCommit ||
        receipt.workflowRunId !== receipt.runner.runId ||
        receipt.workflowRunAttempt !== Number(receipt.runner.runAttempt) ||
        receipt.workflowRunId !== receipt.calibration.dispatchDescriptor.workflowRunId ||
        receipt.workflowRunAttempt !== receipt.calibration.dispatchDescriptor.workflowRunAttempt
      ) {
        throw new Error('External calibration workflow or commit binding mismatch');
      }
    }
  }
  if (new Set(ordered.map((item) => item.clientId)).size !== 2) {
    throw new Error('External vantage client IDs are not distinct');
  }
  if (new Set(ordered.map((item) => item.runner.runnerIdentitySha256)).size !== 2) {
    throw new Error('External vantage runner identities are not distinct');
  }
  if (new Set(ordered.map((item) => item.mode)).size !== 1) {
    throw new Error('External vantage probe modes conflict');
  }
  const paceProfiles = ordered.map((item) => item.paceProfile).sort();
  if (paceProfiles.join(',') !== 'human_controller,slow_concurrency') {
    throw new Error('External vantage receipts do not cover the two frozen pace profiles');
  }
  if (ordered[0].mode === 'calibration') {
    for (const receipt of ordered) {
      validateCalibrationReceipt(receipt.calibration);
      validateDistributedActions(receipt);
    }
    if (new Set(ordered.map((receipt) => receipt.calibration.dispatchDescriptor.descriptorSha256)).size !== 1) {
      throw new Error('External C01 calibration dispatch descriptors conflict');
    }
    const actionKeys = ordered.flatMap((receipt) =>
      receipt.actions.map((action) => `${action.kind}:${action.ordinal}`));
    if (new Set(actionKeys).size !== 60) {
      throw new Error('External C01 calibration action ordinals are not globally unique');
    }
  }
  if (new Set(ordered.map((item) => item.handoff.urlSha256)).size !== 1) {
    throw new Error('External vantage clients did not use the same durable handoff');
  }
  const w8ManifestDigests = [...new Set(ordered
    .map((item) => item.w8ActionManifestSha256)
    .filter(Boolean))];
  if (w8ManifestDigests.length > 1 ||
      (w8ManifestDigests.length === 1 && ordered.some((item) => !item.w8ActionManifestSha256))) {
    throw new Error('External vantage clients have conflicting W8 action-manifest bindings');
  }
  let w8ActionObservations = [];
  if (w8ManifestDigests.length === 1) {
    const actionIdSets = ordered.map((receipt) => {
      const observations = receipt.w8ActionObservations;
      if (!Array.isArray(observations) || observations.length !== 4 ||
          new Set(observations.map((entry) => entry.actionId)).size !== 4 ||
          observations.some((entry) => entry.caseId !== 'H01' ||
            entry.clientId !== receipt.clientId || entry.retryAttempted !== false ||
            entry.repairAttempted !== false || !entry.observedAt || !entry.eventKind)) {
        throw new Error(`External vantage receipt lacks exact H01 observations: ${receipt.clientId}`);
      }
      const orderedTimes = ['open', 'interact', 'disconnect', 'reopen'].map((runnerAction) =>
        Date.parse(observations.find((entry) => entry.runnerAction === runnerAction)?.observedAt));
      if (orderedTimes.some((value) => !Number.isFinite(value)) ||
          orderedTimes.some((value, index) => index > 0 && value < orderedTimes[index - 1])) {
        throw new Error(`External H01 event order is invalid: ${receipt.clientId}`);
      }
      return observations.map((entry) => entry.actionId).sort();
    });
    if (actionIdSets[0].join(',') !== actionIdSets[1].join(',')) {
      throw new Error('External H01 runners observed different action sets');
    }
    w8ActionObservations = actionIdSets[0].map((actionId) => ({
      actionId,
      observations: ordered.map((receipt) => receipt.w8ActionObservations
        .find((entry) => entry.actionId === actionId)),
    }));
  }
  if (
    ordered[0].mode === 'calibration' &&
    ordered.some((receipt) =>
      receipt.calibration.dispatchDescriptor.handoffUrlSha256 !== receipt.handoff.urlSha256)
  ) {
    throw new Error('External calibration descriptor handoff binding mismatch');
  }
  const expectedIdentity = ordered[0].expectedIdentity;
  for (const receipt of ordered) {
    if (canonicalHash(receipt.expectedIdentity) !== canonicalHash(expectedIdentity)) {
      throw new Error('External vantage expected identities conflict');
    }
    for (const observation of [receipt.initialIdentity, receipt.reconnectIdentity]) {
      for (const field of REQUIRED_IDENTITY_FIELDS) {
        if (observation[field] !== expectedIdentity[field]) {
          throw new Error(`External vantage identity mismatch for ${field}`);
        }
      }
    }
    if (receipt.serverPhysicalBrowserLaunchDelta !== 0) {
      throw new Error('External vantage observed a duplicate server browser launch');
    }
    if (receipt.internalUrlLeakCount !== 0 || receipt.oracle.passed !== true) {
      throw new Error('External vantage observed unsafe URL or handoff evidence');
    }
    const ingressKinds = receipt.ingressChecks
      ?.filter((check) => check.state === 'passed')
      .map((check) => check.kind)
      .sort();
    if (ingressKinds?.join(',') !== [...REQUIRED_INGRESS_KINDS].sort().join(',')) {
      throw new Error('External vantage receipt does not prove all eight ingress classes');
    }
    requireCaptureArtifacts(receipt.artifacts ?? []);
    if (
      receipt.visualFixtureAttestation?.syntheticOnly !== true ||
      receipt.visualFixtureAttestation?.forbiddenPrivateFieldsExcluded !== true
    ) {
      throw new Error('External vantage receipt lacks its visual redaction boundary');
    }
  }
  const aggregate = {
    schemaVersion: EXTERNAL_VANTAGE_AGGREGATE_SCHEMA,
    planId: 'P158',
    runId,
    success: true,
    repairAttempted: false,
    retryCount: 0,
    mode: ordered[0].mode,
    clientIds: ordered.map((item) => item.clientId),
    runnerIdentitySha256s: ordered.map((item) => item.runner.runnerIdentitySha256),
    handoffUrlSha256: ordered[0].handoff.urlSha256,
    retainedIdentitySha256: canonicalHash(expectedIdentity),
    w8ActionManifestSha256: w8ManifestDigests[0] ?? null,
    w8ActionObservations,
    receiptSha256s: ordered.map((item) => canonicalHash(item)),
    checks: {
      distinctOffHostClients: true,
      sameDurableHandoff: true,
      exactRetainedIdentity: true,
      noDuplicateServerBrowserLaunch: true,
      noInternalUrlLeak: true,
      allIngressChecks: true,
      calibrationComplete: ordered[0].mode === 'calibration'
        ? ordered.reduce((sum, item) => sum + item.calibration.actionCount, 0) === 50 &&
          ordered.reduce((sum, item) => sum + item.calibration.reconnectCount, 0) === 10
        : null,
      sharedCalibrationWindow: ordered[0].mode === 'calibration'
        ? ordered[0].calibration.dispatchDescriptor.descriptorSha256
        : null,
    },
  };
  return { ...aggregate, aggregateSha256: canonicalHash(aggregate) };
}

function validateW8ProbeManifest(manifest, env, mode) {
  if (!manifest) {
    if (env.P158_W8_ACTION_MANIFEST_SHA256) {
      throw new Error('W8 action manifest bytes are required when its digest is bound');
    }
    return null;
  }
  const { manifestSha256, ...body } = manifest;
  if (mode !== 'readiness' || manifestSha256 !== campaignSha256(body) ||
      manifestSha256 !== env.P158_W8_ACTION_MANIFEST_SHA256 ||
      manifest.schemaVersion !== 'agent-browser.p158-w8-external-action-manifest.v1' ||
      manifest.caseIds?.join(',') !== 'H01' || manifest.actionCount !== 4 ||
      !Array.isArray(manifest.actions) || manifest.actions.length !== 4 ||
      new Set(manifest.actions.map((action) => action.actionId)).size !== 4 ||
      manifest.actions.some((action) => action.caseId !== 'H01' ||
        action.executorKind !== 'external_vantage_aggregate_projection')) {
    throw new Error('External probe W8 manifest is not the exact sealed H01 action set');
  }
  const runnerActions = manifest.actions.map((action) => action.assignment?.runner_action);
  if (runnerActions.join(',') !== 'open,interact,disconnect,reopen') {
    throw new Error('External probe W8 H01 actions are incomplete or reordered');
  }
  return manifest;
}

function buildW8H01ActionObservations({
  manifest, clientId, viewerId, initial, reconnect, disconnectObservedAt,
}) {
  if (!disconnectObservedAt) throw new Error('H01 disconnect event was not observed');
  const evidence = {
    open: {
      observedAt: initial.readyAt,
      eventKind: 'page_open_ready',
      evidenceArtifactId: initial.screenshot.artifactId,
    },
    interact: {
      observedAt: initial.firstUsablePixelsAt,
      eventKind: 'human_paced_interaction_completed',
      evidenceArtifactId: initial.screenshot.artifactId,
    },
    disconnect: {
      observedAt: disconnectObservedAt,
      eventKind: 'playwright_page_closed',
      evidenceArtifactId: initial.screenshot.artifactId,
    },
    reopen: {
      observedAt: reconnect.readyAt,
      eventKind: 'same_handoff_reopened_ready',
      evidenceArtifactId: reconnect.screenshot.artifactId,
    },
  };
  return manifest.actions.map((action) => ({
    actionId: action.actionId,
    attemptId: action.attemptId,
    caseId: action.caseId,
    runnerAction: action.assignment.runner_action,
    clientId,
    viewerId,
    ...evidence[action.assignment.runner_action],
    handoffContinuityObserved: true,
    retainedIdentityObserved: true,
    retryAttempted: false,
    repairAttempted: false,
  }));
}

export function executeP158W8ExternalActionManifest({
  manifest,
  externalVantageAggregate,
  publicHandoffUrl,
  observedAt = new Date().toISOString(),
}) {
  const { manifestSha256, ...manifestBody } = manifest ?? {};
  if (manifest?.schemaVersion !== 'agent-browser.p158-w8-external-action-manifest.v1' ||
      manifestSha256 !== campaignSha256(manifestBody) || !Array.isArray(manifest.actions) ||
      manifest.actionCount !== manifest.actions.length ||
      new Set(manifest.actions.map((action) => action.actionId)).size !== manifest.actions.length ||
      manifest.repairAllowed !== false || manifest.retryAllowed !== false ||
      manifest.garbageCollectionAllowed !== false) {
    throw new Error('W8 external action manifest is missing, changed, or duplicates an action');
  }
  const aggregate = externalVantageAggregate;
  if (aggregate?.schemaVersion !== EXTERNAL_VANTAGE_AGGREGATE_SCHEMA || aggregate.success !== true ||
      aggregate.aggregateSha256 !== canonicalHash(withoutKey(aggregate, 'aggregateSha256')) ||
      aggregate.handoffUrlSha256 !== manifest.handoffUrlSha256 ||
      aggregate.clientIds?.length !== 2 || new Set(aggregate.clientIds).size !== 2 ||
      aggregate.checks?.distinctOffHostClients !== true || aggregate.checks?.sameDurableHandoff !== true ||
      aggregate.checks?.exactRetainedIdentity !== true ||
      aggregate.checks?.noDuplicateServerBrowserLaunch !== true ||
      aggregate.checks?.noInternalUrlLeak !== true || aggregate.checks?.allIngressChecks !== true ||
      aggregate.w8ActionManifestSha256 !== manifestSha256) {
    throw new Error('W8 action execution lacks its exact clean two-runner external-vantage aggregate');
  }
  if (hashText(publicHandoffUrl) !== manifest.handoffUrlSha256) {
    throw new Error('W8 public handoff URL does not match its sealed digest');
  }
  const actionReceipts = manifest.actions.map((action) => {
    if (action.caseId !== 'H01') {
      throw new Error(`W8 external action executor does not implement ${action.caseId}`);
    }
    const observed = aggregate.w8ActionObservations?.find((entry) => entry.actionId === action.actionId);
    const evidence = {
      runnerAction: action.assignment?.runner_action,
      clientIds: [...aggregate.clientIds],
      sameDurableHandoff: true,
      exactRetainedIdentity: true,
      observations: structuredClone(observed?.observations ?? []),
    };
    if (!['open', 'interact', 'disconnect', 'reopen'].includes(evidence.runnerAction) ||
        evidence.observations.length !== 2 ||
        evidence.observations.some((entry) => entry.runnerAction !== evidence.runnerAction ||
          entry.handoffContinuityObserved !== true || entry.retainedIdentityObserved !== true)) {
      throw new Error(`W8 H01 action ${action.actionId} is not a frozen runner action`);
    }
    const body = {
      schemaVersion: 'agent-browser.p158-w8-external-action-receipt.v1',
      planId: 'P158',
      actionId: action.actionId,
      attemptId: action.attemptId,
      caseId: action.caseId,
      environmentIds: [...action.environmentIds],
      candidateSha256: manifest.candidateSha256,
      workflowSha256: manifest.workflowSha256,
      manifestSha256,
      externalVantageAggregateSha256: aggregate.aggregateSha256,
      handoffUrlSha256: manifest.handoffUrlSha256,
      terminalState: 'completed',
      resultState: 'passed',
      attemptNumber: 1,
      observedAt,
      evidence,
      repairAttempted: false,
      retryAttempted: false,
      garbageCollectionAttempted: false,
      privateContentCaptured: false,
      secretInputCaptured: false,
    };
    return { ...body, receiptSha256: campaignSha256(body) };
  });
  const result = {
    schemaVersion: W8_EXTERNAL_ACTION_RESULT_SCHEMA,
    planId: 'P158',
    manifestSha256,
    externalVantageAggregateSha256: aggregate.aggregateSha256,
    actionCount: actionReceipts.length,
    actionReceipts,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  };
  return { ...result, resultSha256: campaignSha256(result) };
}

function validateCalibrationReceipt(calibration) {
  if (
    calibration?.actualDurationMs < 20 * 60 * 1000 ||
    calibration?.actionCount !== 25 ||
    calibration?.reconnectCount !== 5
  ) {
    throw new Error('External C01 calibration counts or duration are incomplete');
  }
  const expected = buildExternalCalibrationSchedule();
  const descriptor = calibration.dispatchDescriptor;
  if (
    descriptor?.durationMs !== CALIBRATION_DURATION_MS ||
    descriptor?.lateToleranceMs !== CALIBRATION_LATE_TOLERANCE_MS ||
    descriptor?.scheduleSha256 !== canonicalHash(expected) ||
    descriptor?.descriptorSha256 !== canonicalHash(withoutKey(descriptor, 'descriptorSha256'))
  ) {
    throw new Error('External C01 calibration dispatch descriptor is invalid');
  }
  if (
    !Number.isFinite(calibration.runnerStartDelayMs) ||
    calibration.runnerStartDelayMs > CALIBRATION_LATE_TOLERANCE_MS ||
    calibration.runnerQueueDelayMs !== Math.max(0, calibration.runnerStartDelayMs)
  ) {
    throw new Error('External C01 runner began beyond the frozen lateness tolerance');
  }
  if (!Array.isArray(calibration.events) || calibration.events.length !== expected.length) {
    throw new Error('External C01 calibration event ledger is incomplete');
  }
  for (let index = 0; index < expected.length; index += 1) {
    const observed = calibration.events[index];
    const scheduled = expected[index];
    const observedMs = Date.parse(observed.observedAt);
    const scheduledMs = Date.parse(descriptor.calibrationStartAt) + scheduled.offsetMs;
    if (
      observed.kind !== scheduled.kind ||
      observed.ordinal !== scheduled.ordinal ||
      observed.offsetMs !== scheduled.offsetMs ||
      !Number.isFinite(observedMs) ||
      observedMs < scheduledMs ||
      observedMs > Date.parse(descriptor.calibrationEndAt)
    ) {
      throw new Error(`External C01 calibration event ${index + 1} does not match the frozen schedule`);
    }
    if (
      index > 0 &&
      Date.parse(observed.observedAt) < Date.parse(calibration.events[index - 1].observedAt)
    ) {
      throw new Error('External C01 calibration timestamps are not monotonic');
    }
  }
}

function validateDistributedActions(receipt) {
  if (!Array.isArray(receipt.actions) || receipt.actions.length !== 30) {
    throw new Error('External C01 receipt must contain exactly 30 client actions');
  }
  const expectedCounts = { dashboard_action: 25, handoff_reconnect: 5 };
  for (const [kind, expectedCount] of Object.entries(expectedCounts)) {
    const actions = receipt.actions.filter((action) => action.kind === kind);
    if (actions.length !== expectedCount) {
      throw new Error(`External C01 receipt has an invalid ${kind} count`);
    }
    for (const action of actions) {
      if (
        action.viewerId !== receipt.viewerId ||
        action.attempt !== 1 ||
        action.state !== 'passed' ||
        action.retryAttempted !== false ||
        action.repairAttempted !== false ||
        !Number.isFinite(action.latencyMs) ||
        action.latencyMs < 0 ||
        !Number.isFinite(Date.parse(action.observedAt))
      ) {
        throw new Error(`External C01 ${kind} terminal evidence is invalid`);
      }
    }
  }
}

export async function runExternalVantageProbe({
  env,
  clientId,
  paceProfile,
  mode = 'readiness',
  outputDir,
  w8ActionManifest = null,
}) {
  mkdirSync(outputDir, { recursive: true });
  try {
    return await executeExternalVantageProbe({
      env, clientId, paceProfile, mode, outputDir, w8ActionManifest,
    });
  } catch (error) {
    const failureRecord = externalVantageFailureRecord(error, env);
    const failure = {
      schemaVersion: EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
      planId: 'P158',
      runId: env.P158_RUN_ID || null,
      clientId: clientId || null,
      paceProfile: paceProfile || null,
      mode,
      success: false,
      repairAttempted: false,
      retryCount: 0,
      failedAt: new Date().toISOString(),
      failure: failureRecord,
      calibrationTiming: failureCalibrationTiming(env),
      artifacts: artifactReceipts(outputDir),
    };
    const serialized = `${JSON.stringify(failure, null, 2)}\n`;
    assertSecretsAbsent(serialized, env);
    writeFileSync(join(outputDir, 'failure-receipt.json'), serialized, { mode: 0o600 });
    throw error;
  }
}

async function executeExternalVantageProbe({
  env, clientId, paceProfile, mode, outputDir, w8ActionManifest,
}) {
  const {
    handoff,
    expectedIdentity: configuredIdentity,
    pixelMarkerRegion,
    visualFixtureAttestation,
    calibrationDescriptor,
  } =
    validateExternalVantageConfiguration({ env, clientId, paceProfile, mode });
  if (mode === 'calibration') validateExternalCalibrationLeadTime(calibrationDescriptor);
  const reviewedW8Manifest = validateW8ProbeManifest(w8ActionManifest, env, mode);
  const startedAt = new Date().toISOString();
  const urlEvidence = [];
  const networkEntries = [];
  const consoleEntries = [];
  const handoffResolutions = [];
  const resolvedAddresses = await resolvePublicAddresses(handoff.hostname);
  const tls = await observeTls(handoff);
  const initialClassification = classifyOperatorUrl(handoff.href, {
    role: 'starting_handoff',
    resolvedAddresses,
  });
  if (!initialClassification.valid || initialClassification.findingCodes.length) {
    throw new Error('External DNS resolution or HTTPS classification is not public');
  }
  const browser = await chromium.launch({ headless: true });
  const browserVersion = browser.version();
  let context;
  try {
    context = await browser.newContext({
      viewport: { width: 1440, height: 1000 },
      reducedMotion: 'reduce',
      locale: 'en-US',
      recordVideo: { dir: join(outputDir, 'video'), size: { width: 1440, height: 1000 } },
    });
    const auth = await context.request.post(new URL('/api/dashboard-auth/login', handoff.origin).href, {
      data: {
        username: env.P158_DEV_DASHBOARD_USERNAME,
        password: env.P158_DEV_DASHBOARD_PASSWORD,
      },
      failOnStatusCode: false,
    });
    if (!auth.ok()) {
      throw new Error(`Dashboard authentication failed with HTTP ${auth.status()}`);
    }
    const authStatus = await context.request.get(new URL('/api/dashboard-auth/status', handoff.origin).href, {
      failOnStatusCode: false,
    });
    if (!authStatus.ok()) {
      throw new Error(`Dashboard authenticated session verification failed with HTTP ${authStatus.status()}`);
    }
    const cookieMetadata = (await context.cookies(handoff.origin)).map((cookie) => ({
      nameSha256: hashText(cookie.name),
      domain: cookie.domain,
      path: cookie.path,
      secure: cookie.secure,
      httpOnly: cookie.httpOnly,
      sameSite: cookie.sameSite,
      expires: cookie.expires,
    }));
    if (!cookieMetadata.some((cookie) => cookie.secure && cookie.httpOnly)) {
      throw new Error('Authenticated session cookie lacks secure and HttpOnly metadata');
    }
    let page = await context.newPage();
    attachCapture(page, { urlEvidence, networkEntries, consoleEntries, handoffResolutions });
    const initial = await captureVisit({
      page,
      handoff,
      expectedIdentity: configuredIdentity,
      outputDir,
      label: 'initial',
      paceProfile,
      urlEvidence,
      handoffResolutions,
      pixelMarkerRegion,
      performHumanAction: mode === 'readiness',
    });
    let concurrency = null;
    let concurrentPage = null;
    if (paceProfile === 'slow_concurrency') {
      concurrentPage = await context.newPage();
      attachCapture(concurrentPage, { urlEvidence, networkEntries, consoleEntries, handoffResolutions });
      concurrency = await captureVisit({
        page: concurrentPage,
        handoff,
        expectedIdentity: configuredIdentity,
        outputDir,
        label: 'concurrent',
        paceProfile,
        urlEvidence,
        handoffResolutions,
        pixelMarkerRegion,
        performHumanAction: false,
      });
      assertSameIdentity(initial.identity, concurrency.identity, 'concurrent client page');
    }
    const calibration = {
      mode,
      requiredDurationMs: mode === 'calibration' ? CALIBRATION_DURATION_MS : 0,
      actionCount: 0,
      reconnectCount: 0,
      events: [],
    };
    let reconnect = null;
    let disconnectObservedAt = null;
    const reconnectObservations = [];
    if (mode === 'calibration') {
      const calibrationStartMs = Date.parse(calibrationDescriptor.calibrationStartAt);
      const runnerReadyMs = Date.now();
      const runnerStartDelayMs = runnerReadyMs - calibrationStartMs;
      calibration.dispatchDescriptor = calibrationDescriptor;
      calibration.runnerReadyAt = new Date(runnerReadyMs).toISOString();
      calibration.runnerStartDelayMs = runnerStartDelayMs;
      calibration.runnerQueueDelayMs = Math.max(0, runnerStartDelayMs);
      if (runnerStartDelayMs > CALIBRATION_LATE_TOLERANCE_MS) {
        throw new Error('External runner reached the shared calibration barrier too late');
      }
      if (runnerStartDelayMs < 0) {
        await new Promise((resolvePromise) => setTimeout(resolvePromise, -runnerStartDelayMs));
      }
      for (const event of buildExternalCalibrationSchedule()) {
        const remainingMs = calibrationStartMs + event.offsetMs - Date.now();
        if (remainingMs > 0) await new Promise((resolvePromise) => setTimeout(resolvePromise, remainingMs));
        if (event.kind === 'dashboard_action') {
          const actionStartedMs = Date.now();
          await humanPacedObservation(page, paceProfile, pixelMarkerRegion);
          calibration.actionCount += 1;
          calibration.events.push({
            ...event,
            observedAt: new Date().toISOString(),
            latencyMs: Date.now() - actionStartedMs,
          });
          continue;
        }
        await page.close();
        page = await context.newPage();
        attachCapture(page, { urlEvidence, networkEntries, consoleEntries, handoffResolutions });
        reconnect = await captureVisit({
          page,
          handoff,
          expectedIdentity: configuredIdentity,
          outputDir,
          label: `reconnect-${event.ordinal}`,
          paceProfile,
          urlEvidence,
          handoffResolutions,
          pixelMarkerRegion,
          performHumanAction: false,
        });
        assertSameIdentity(initial.identity, reconnect.identity, `scheduled reconnect ${event.ordinal}`);
        calibration.reconnectCount += 1;
        calibration.events.push({
          ...event,
          observedAt: new Date().toISOString(),
          latencyMs: reconnect.durationMs,
        });
        reconnectObservations.push(reconnect);
      }
      const untilSharedEndMs = calibrationStartMs + CALIBRATION_DURATION_MS - Date.now();
      if (untilSharedEndMs > 0) {
        await new Promise((resolvePromise) => setTimeout(resolvePromise, untilSharedEndMs));
      }
      calibration.actualDurationMs = Date.now() - calibrationStartMs;
      if (calibration.actualDurationMs < calibration.requiredDurationMs) {
        throw new Error('External calibration ended before its frozen 20-minute duration');
      }
    } else {
      await page.close();
      disconnectObservedAt = new Date().toISOString();
      page = await context.newPage();
      attachCapture(page, { urlEvidence, networkEntries, consoleEntries, handoffResolutions });
      reconnect = await captureVisit({
        page,
        handoff,
        expectedIdentity: configuredIdentity,
        outputDir,
        label: 'reconnect',
        paceProfile,
        urlEvidence,
        handoffResolutions,
        pixelMarkerRegion,
        performHumanAction: true,
      });
      assertSameIdentity(initial.identity, reconnect.identity, 'scheduled reconnect');
      reconnectObservations.push(reconnect);
      calibration.reconnectCount = 1;
    }
    if (!reconnect) throw new Error('Scheduled reconnect evidence was not captured');
    const performance = await capturePerformance(page);
    await page.close();
    await concurrentPage?.close();
    const initialServiceBrowser = initial.serviceBrowserObservation;
    const reconnectServiceBrowser = reconnect.serviceBrowserObservation;
    const serverPhysicalBrowserLaunchDelta =
      initialServiceBrowser.processIdentitySha256 === reconnectServiceBrowser.processIdentitySha256 ? 0 : 1;
    await context.close();
    context = null;
    writeSanitizedHar(networkEntries, join(outputDir, 'network.redacted.har'));
    const leaks = findInternalUrlLeaks(urlEvidence);
    if (leaks.length) {
      const error = new Error(`Internal URL evidence detected in ${leaks.length} observations`);
      error.code = 'external_url_policy_violation';
      error.details = {
        urlFindingCount: leaks.length,
        urlFindingRoles: [...new Set(leaks.map((finding) => finding.role))].sort(),
        urlFindingCodes: [...new Set(leaks.flatMap((finding) => finding.findingCodes))].sort(),
      };
      throw error;
    }
    const expectedIdentity = { ...configuredIdentity, pixelHash: initial.identity.pixelHash };
    if (configuredIdentity.pixelHash && configuredIdentity.pixelHash !== initial.identity.pixelHash) {
      throw new Error('Configured pixel hash does not match captured pixels');
    }
    const oracleSession = buildOracleSession({
      clientId,
      handoff,
      resolvedAddresses,
      tls,
      initial,
      reconnect,
      reconnectObservations,
      expectedIdentity,
      urlEvidence,
      cookieMetadata,
      serverPhysicalBrowserLaunchDelta,
    });
    const oracle = auditExternalHandoffSession({ session: oracleSession });
    if (!oracle.passed) {
      const findingCodes = [...new Set(oracle.findings.map((item) => item.code))].sort();
      const error = new Error(`External handoff oracle rejected evidence: ${findingCodes.join(',')}`);
      error.code = 'external_handoff_oracle_rejected';
      error.details = {
        urlFindingCount: oracle.findings.length,
        urlFindingRoles: [...new Set(oracle.findings
          .map((item) => item.observed?.role)
          .filter((role) => EXTERNAL_URL_ROLES.includes(role)))].sort(),
        urlFindingCodes: findingCodes,
        iframeCount: oracleSession.surfaceScans?.iframeUrlCount
          ?? oracleSession.urlObservations.filter((item) => item.role === 'iframe_src').length,
      };
      throw error;
    }
    const viewerId = `external-viewer-${clientId.replace(/^external-runner-/, '')}`;
    const w8ActionObservations = reviewedW8Manifest
      ? buildW8H01ActionObservations({
          manifest: reviewedW8Manifest,
          clientId,
          viewerId,
          initial,
          reconnect,
          disconnectObservedAt,
        })
      : [];
    const receiptBody = {
      schemaVersion: mode === 'calibration'
        ? EXTERNAL_CALIBRATION_RECEIPT_SCHEMA
        : EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
      planId: 'P158',
      runId: env.P158_RUN_ID,
      receiptId: `${env.P158_RUN_ID}-${clientId}-${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT}`,
      clientId,
      viewerId,
      paceProfile,
      mode,
      success: true,
      repairAttempted: false,
      retryCount: 0,
      startedAt: mode === 'calibration' ? calibrationDescriptor.calibrationStartAt : startedAt,
      completedAt: mode === 'calibration' ? calibrationDescriptor.calibrationEndAt : new Date().toISOString(),
      probeStartedAt: startedAt,
      sourceCommit: env.P158_CANDIDATE_COMMIT || null,
      workflowRunId: env.GITHUB_RUN_ID,
      workflowRunAttempt: Number(env.GITHUB_RUN_ATTEMPT),
      w8ActionManifestSha256: env.P158_W8_ACTION_MANIFEST_SHA256 || null,
      w8ActionObservations,
      runner: runnerEvidence(env),
      runnerIdentity: distributedRunnerIdentity(env),
      outsideServiceHost: true,
      outsideServiceNetworkNamespace: true,
      publicEgressObserved: true,
      toolchain: {
        nodeVersion: process.version,
        playwrightVersion: PINNED_PLAYWRIGHT_VERSION,
        chromiumVersion: browserVersion,
      },
      externalVantage: oracleSession.externalVantage,
      handoff: {
        origin: handoff.origin,
        pathTemplate: '/remote-view/<redacted>',
        urlSha256: hashText(handoff.href),
        handoffIdSha256: hashText(oracleSession.initialHandoffId),
      },
      dns: { hostname: handoff.hostname, resolvedAddresses },
      tls,
      cookieMetadata,
      ingressChecks: oracleSession.ingressChecks.map(sanitizeIngressCheck),
      urlEvidence: urlEvidence.map(sanitizeUrlEvidence),
      internalUrlLeakCount: leaks.length,
      expectedIdentity,
      initialIdentity: initial.identity,
      reconnectIdentity: reconnect.identity,
      concurrencyIdentity: concurrency?.identity ?? null,
      serverPhysicalBrowserLaunchDelta,
      serviceBrowserObservations: [initialServiceBrowser, reconnectServiceBrowser],
      consoleEntries,
      networkEntries,
      performance,
      calibration,
      actions: mode === 'calibration'
        ? calibration.events.map((event) => ({
          kind: event.kind,
          ordinal: globalCalibrationOrdinal(event.kind, event.ordinal, paceProfile),
          viewerId,
          attempt: 1,
          state: 'passed',
          observedAt: event.observedAt,
          latencyMs: event.latencyMs,
          retryAttempted: false,
          repairAttempted: false,
        }))
        : [],
      visualFixtureAttestation,
      surfaceScans: {
        redirectLocationCount: urlEvidence.filter((item) => item.role === 'location_header').length,
        iframeUrlCount: urlEvidence.filter((item) => item.role === 'iframe_src').length,
        formActionUrlCount: urlEvidence.filter((item) => item.role === 'form_action').length,
        websocketUrlCount: urlEvidence.filter((item) => item.role === 'websocket_endpoint').length,
        reconnectUrlCount: urlEvidence.filter((item) => item.role === 'reconnect_target').length,
        copiedActionUrlCount: urlEvidence.filter((item) => item.role === 'copied_action').length,
        errorActionUrlCount: urlEvidence.filter((item) => item.role === 'error_action').length,
      },
      artifacts: requireCaptureArtifacts(artifactReceipts(outputDir)),
      oracle: {
        passed: oracle.passed,
        reportSha256: canonicalHash(oracle),
        findingCodes: oracle.findings.map((item) => item.code),
      },
    };
    const receipt = mode === 'calibration'
      ? { ...receiptBody, receiptSha256: campaignSha256(receiptBody) }
      : receiptBody;
    const serialized = `${JSON.stringify(receipt, null, 2)}\n`;
    assertSecretsAbsent(serialized, env);
    writeFileSync(join(outputDir, 'receipt.json'), serialized, { mode: 0o600 });
    return receipt;
  } catch (error) {
    writeSanitizedHar(networkEntries, join(outputDir, 'network.redacted.har'));
    const guacamoleEntries = networkEntries.filter((entry) => {
      try {
        return new URL(entry.url).pathname.startsWith('/guacamole');
      } catch {
        return false;
      }
    });
    error.details = {
      ...(error?.details && typeof error.details === 'object' ? error.details : {}),
      networkEntryCount: networkEntries.length,
      guacamoleNetworkEntryCount: guacamoleEntries.length,
      guacamoleHttpStatusCounts: Object.fromEntries(
        [...new Set(guacamoleEntries.map((entry) => entry.status))]
          .sort((left, right) => left - right)
          .map((status) => [String(status), guacamoleEntries.filter((entry) => entry.status === status).length]),
      ),
      websocketObservationCount: urlEvidence.filter((entry) => entry.role === 'websocket_endpoint').length,
      consoleEntryCount: consoleEntries.length,
      resolutionObservationCount: handoffResolutions.length,
    };
    throw error;
  } finally {
    await context?.close().catch(() => {});
    await browser.close().catch(() => {});
  }
}

async function captureVisit({ page, handoff, expectedIdentity, outputDir, label, paceProfile, urlEvidence, handoffResolutions, pixelMarkerRegion, performHumanAction }) {
  const began = Date.now();
  const resolutionStartIndex = handoffResolutions.length;
  const response = await page.goto(handoff.href, { waitUntil: 'domcontentloaded', timeout: 45_000 });
  if (!response) throw new Error(`${label} navigation returned no response`);
  const resolution = await waitForAuthoritativeHandoffResolution(
    handoffResolutions,
    resolutionStartIndex,
    30_000,
  );
  const readyAt = new Date().toISOString();
  if (performHumanAction) await humanPacedObservation(page, paceProfile, pixelMarkerRegion);
  const screenshotPath = join(outputDir, `${label}.png`);
  const markerPath = join(outputDir, `${label}-pixel-marker.png`);
  const pixelHash = await waitForExpectedPixelMarker({
    page,
    screenshotPath,
    markerPath,
    pixelMarkerRegion,
    expectedPixelHash: expectedIdentity.pixelHash,
  });
  // Bind the DOM evidence to the same converged presentation that supplied
  // the accepted pixels. Sampling before convergence can observe the brief
  // React replacement between the placeholder and its iframe.
  const iframeUrls = await page.locator('iframe').evaluateAll((frames) => frames.map((frame) => frame.src).filter(Boolean));
  const formActions = await page.locator('form').evaluateAll((forms) => forms.map((form) => form.action).filter(Boolean));
  const copiedActions = await page.locator('a,button').evaluateAll((elements) => elements.flatMap((element) => {
    const elementLabel = `${element.textContent || ''} ${element.getAttribute('aria-label') || ''}`;
    if (!/copy/i.test(elementLabel)) return [];
    const value = element.getAttribute('data-copy-url') || element.getAttribute('href');
    return value ? [new URL(value, document.baseURI).href] : [];
  }));
  const errorActions = await page.locator('a').evaluateAll((elements) => elements.flatMap((element) => {
    if (!element.closest('[role="alert"],.workspace-remote-viewport-notice-bad')) return [];
    return element.href ? [element.href] : [];
  }));
  iframeUrls.forEach((url, index) => urlEvidence.push({ evidenceId: `${label}-iframe-${index}`, role: 'iframe_src', url }));
  formActions.forEach((url, index) => urlEvidence.push({ evidenceId: `${label}-form-${index}`, role: 'form_action', url }));
  copiedActions.forEach((url, index) => urlEvidence.push({ evidenceId: `${label}-copied-${index}`, role: 'copied_action', url }));
  errorActions.forEach((url, index) => urlEvidence.push({ evidenceId: `${label}-error-${index}`, role: 'error_action', url }));
  if (label.startsWith('reconnect')) {
    urlEvidence.push({ evidenceId: `${label}-target`, role: 'reconnect_target', url: handoff.href });
  }
  await page.screenshot({ path: screenshotPath, fullPage: false });
  const identity = identityFromResolution(resolution, expectedIdentity);
  identity.pixelHash = pixelHash;
  const screenshot = {
    artifactId: `${label}-screenshot`,
    file: basename(screenshotPath),
    sha256: sha256File(screenshotPath),
  };
  return {
    label,
    readyAt,
    firstUsablePixelsAt: new Date().toISOString(),
    readiness: { state: 'ready' },
    identity,
    screenshot,
    durationMs: Date.now() - began,
    serverPhysicalBrowserLaunchDelta: 0,
    navigation: {
      status: response.status(),
      finalUrl: response.url(),
      redirectCount: redirectChainLength(response.request()),
    },
    formActionCount: formActions.length,
    formSurfaceAbsent: formActions.length === 0,
    serviceBrowserObservation: serviceBrowserObservationFromResolution(resolution, readyAt),
  };
}

async function waitForExpectedPixelMarker({
  page,
  screenshotPath,
  markerPath,
  pixelMarkerRegion,
  expectedPixelHash,
  timeoutMs = 20_000,
}) {
  const deadline = Date.now() + timeoutMs;
  let observedPixelHash = null;
  do {
    let screenshotClip = pixelMarkerRegion;
    if (pixelMarkerRegion.coordinateSpace === 'remote-view-iframe') {
      const frames = page.locator('iframe');
      const frameCount = await frames.count();
      const iframeBox = frameCount === 1 ? await frames.first().boundingBox() : null;
      screenshotClip = remoteViewIframeClipObservation(pixelMarkerRegion, frameCount, iframeBox);
      if (!screenshotClip) {
        await page.waitForTimeout(500);
        continue;
      }
    }
    await page.screenshot({ path: markerPath, clip: screenshotClip });
    observedPixelHash = sha256File(markerPath);
    if (observedPixelHash === expectedPixelHash) return observedPixelHash;
    await page.waitForTimeout(500);
  } while (Date.now() < deadline);

  await page.screenshot({ path: screenshotPath, fullPage: false });
  const diagnostic = await renderedStreamDiagnostic(page, observedPixelHash);
  const error = new Error('The external dashboard did not render the prepared remote pixel marker');
  error.code = diagnostic.code;
  error.details = diagnostic.details;
  throw error;
}

export function remoteViewIframeClipObservation(region, iframeCount, iframeBox) {
  if (iframeCount === 0) return null;
  if (iframeCount !== 1) {
    throw new Error(`Remote pixel marker requires exactly one iframe, observed ${iframeCount}`);
  }
  if (!iframeBox) return null;
  return pixelMarkerClipForIframe(region, iframeBox);
}

export function pixelMarkerClipForIframe(region, iframeBox) {
  const clip = {
    x: iframeBox.x + region.x,
    y: iframeBox.y + region.y,
    width: region.width,
    height: region.height,
  };
  if (
    region.x < 0 || region.y < 0 || region.width < 1 || region.height < 1 ||
    region.x + region.width > iframeBox.width ||
    region.y + region.height > iframeBox.height
  ) {
    throw new Error('Pixel marker region does not fit the rendered remote-view iframe');
  }
  return clip;
}

async function renderedStreamDiagnostic(page, observedPixelHash) {
  const bodyText = (await page.locator('body').innerText().catch(() => '')).slice(0, 20_000);
  const iframePaths = await page.locator('iframe').evaluateAll((frames) => frames.map((frame) => {
    try {
      const url = new URL(frame.src);
      return url.pathname;
    } catch {
      return 'invalid';
    }
  }));
  return classifyRenderedStreamFailure({ bodyText, iframePaths, observedPixelHash });
}

export function classifyRenderedStreamFailure({ bodyText, iframePaths, observedPixelHash }) {
  const normalizedText = String(bodyText || '').toLowerCase();
  const paths = Array.isArray(iframePaths) ? iframePaths : [];
  let code = 'external_stream_identity_marker_missing';
  if (normalizedText.includes('stream sign-in expired')) {
    code = 'external_stream_auth_failed';
  } else if (normalizedText.includes('connecting to cdp stream')) {
    code = 'external_stream_not_rendered';
  } else if (paths.some((path) => path !== '/guacamole/' && path !== '/guacamole')) {
    code = 'external_stream_route_invalid';
  } else if (paths.length === 0) {
    code = 'external_stream_not_embeddable';
  }
  return {
    code,
    details: {
      iframeCount: paths.length,
      iframePathClasses: paths.map((path) => (
        path === '/guacamole/' || path === '/guacamole' ? 'guacamole' : 'unexpected'
      )),
      observedPixelHash,
      streamSignInExpired: normalizedText.includes('stream sign-in expired'),
      cdpStreamConnecting: normalizedText.includes('connecting to cdp stream'),
    },
  };
}

function redirectChainLength(request) {
  let count = 0;
  let cursor = request.redirectedFrom();
  while (cursor) {
    count += 1;
    cursor = cursor.redirectedFrom();
  }
  return count;
}

function attachCapture(page, capture) {
  page.on('console', (message) => {
    capture.consoleEntries.push({
      type: message.type(),
      textSha256: hashText(message.text()),
      timestamp: new Date().toISOString(),
    });
  });
  page.on('websocket', (socket) => {
    capture.urlEvidence.push({
      evidenceId: `websocket-${capture.urlEvidence.length + 1}`,
      role: 'websocket_endpoint',
      url: socket.url(),
    });
  });
  page.on('response', async (response) => {
    const request = response.request();
    const entry = {
      entryId: `network-${capture.networkEntries.length + 1}`,
      url: redactOperatorUrl(response.url()),
      urlSha256: hashText(response.url()),
      method: request.method(),
      resourceType: request.resourceType(),
      status: response.status(),
      timestamp: new Date().toISOString(),
    };
    capture.networkEntries.push(entry);
    const location = response.headers().location;
    if (location) {
      capture.urlEvidence.push({
        evidenceId: `${entry.entryId}-location`,
        role: 'location_header',
        url: new URL(location, response.url()).href,
      });
    }
    const responsePath = new URL(response.url()).pathname;
    if (
      responsePath === '/api/service/request' &&
      request.method() === 'POST' &&
      (response.headers()['content-type'] || '').includes('application/json')
    ) {
      let requestPayload = null;
      try {
        requestPayload = request.postDataJSON();
      } catch {
        requestPayload = null;
      }
      if (requestPayload?.action !== 'service_remote_view_handoff_resolve') return;
      const resolverEnvelope = await response.json().catch(() => null);
      if (!resolverEnvelope || typeof resolverEnvelope !== 'object') {
        return;
      }
      if (resolverEnvelope?.success !== true) {
        const failure = resolverEnvelope.failure && typeof resolverEnvelope.failure === 'object'
          ? resolverEnvelope.failure
          : {};
        const recourse = failure.recourse && typeof failure.recourse === 'object'
          ? failure.recourse
          : failure;
        capture.handoffResolutions.push({
          status: 'failed',
          resolved: false,
          failureCode: safeFailureToken(failure.code ?? recourse.code),
          effectState: safeFailureToken(recourse.effectState),
          retryDisposition: safeFailureToken(recourse.retryDisposition),
          waitMs: Number.isSafeInteger(recourse.waitMs) ? recourse.waitMs : null,
        });
        return;
      }
      if (!resolverEnvelope.data || typeof resolverEnvelope.data !== 'object') return;
      const projected = projectHandoffResolution(resolverEnvelope.data);
      capture.handoffResolutions.push(projected);
      for (const discovered of projected.urlObservations) {
        capture.urlEvidence.push({
          evidenceId: `${entry.entryId}-${discovered.role}-${capture.urlEvidence.length + 1}`,
          ...discovered,
        });
      }
    }
  });
}

export function projectHandoffResolution(data) {
  const tab = data.tab ?? {};
  const handle = tab.serviceTabHandle ?? {};
  const intent = data.open?.intent ?? {};
  const open = data.open ?? {};
  const routeBinding = data.routeBinding ?? open.routeBinding ?? {};
  const routeDescriptor = routeBinding.routeDescriptor ?? {};
  const urlObservations = [
    ['provider_external_url', data.providerExternalUrl],
    ['provider_external_url', open.providerExternalUrl],
    ['route_binding', routeBinding.externalUrl],
    ['route_binding', routeBinding.frameUrl],
    ['local_embed_url', data.localEmbedUrl ?? routeBinding.localEmbedUrl ?? routeDescriptor.localEmbedUrl],
    ['dashboard_embed_url', data.dashboardEmbedUrl ?? routeBinding.dashboardEmbedUrl ?? routeDescriptor.dashboardEmbedUrl],
    ['health_url', data.healthUrl ?? routeBinding.healthUrl ?? routeDescriptor.healthUrl],
    ['copied_action', data.handoffUrl],
    ['reconnect_target', data.reconnectUrl],
    ['error_action', data.errorActionUrl],
  ].flatMap(([role, url]) =>
    typeof url === 'string' && /^[a-z][a-z0-9+.-]*:/i.test(url) ? [{ role, url }] : [],
  );
  return {
    status: data.status ?? null,
    resolved: data.resolved === true,
    reopenRequired: data.reopenRequired === true,
    handoffId: data.handoffId ?? null,
    handoffUrl: data.handoffUrl ?? null,
    browserId: data.browserId ?? null,
    sessionName: data.sessionName ?? null,
    tabId: data.tabId ?? null,
    targetId: data.targetId ?? null,
    viewStreamProvider: data.viewStreamProvider ?? null,
    presentationGeneration: data.presentationGeneration ?? null,
    presentationReceipt: data.presentationReceipt ? {
      generation: data.presentationReceipt.generation ?? null,
      dashboardDeploymentGeneration: data.presentationReceipt.dashboardDeploymentGeneration ?? null,
      logicalBrowserId: data.presentationReceipt.logicalBrowserId ?? null,
      daemonOwnerGeneration: data.presentationReceipt.daemonOwnerGeneration ?? null,
      processInstanceDigest: data.presentationReceipt.processInstanceDigest ?? null,
      targetId: data.presentationReceipt.targetId ?? null,
      requiredStreamProvider: data.presentationReceipt.requiredStreamProvider ?? null,
      observedStreamProvider: data.presentationReceipt.observedStreamProvider ?? null,
      state: data.presentationReceipt.state ?? null,
    } : null,
    tab: {
      id: tab.id ?? null,
      tabId: tab.tabId ?? null,
      browserId: tab.browserId ?? null,
      profileId: tab.profileId ?? null,
      runtimeProfile: tab.runtimeProfile ?? null,
      sessionId: tab.sessionId ?? null,
      targetId: tab.targetId ?? null,
      url: tab.url ?? null,
      title: tab.title ?? null,
      pageMarker: tab.pageMarker ?? null,
      serviceTabHandle: {
        browserId: handle.browserId ?? null,
        profileId: handle.profileId ?? null,
        sessionName: handle.sessionName ?? null,
        tabId: handle.tabId ?? null,
        targetId: handle.targetId ?? null,
      },
    },
    open: { intent: { runtimeProfile: intent.runtimeProfile ?? null, profile: intent.profile ?? null, url: intent.url ?? null } },
    urlObservations,
  };
}

async function humanPacedObservation(page, profile, pixelMarkerRegion) {
  const delay = profile === 'slow_concurrency' ? 900 : 300;
  await page.mouse.move(220, 180, { steps: 8 });
  await page.waitForTimeout(delay);
  await page.keyboard.press('Tab');
  await page.waitForTimeout(delay);
  await page.keyboard.press('Shift+Tab');
  await page.mouse.wheel(0, 240);
  await page.waitForTimeout(delay);
  await page.mouse.wheel(0, -240);
  const remoteFrame = page.locator('iframe').first();
  if (await remoteFrame.count()) {
    const box = await remoteFrame.boundingBox();
    if (box) {
      const point = syntheticRemoteInteractionPoint(pixelMarkerRegion, box);
      await page.mouse.click(point.x, point.y);
      await page.waitForTimeout(delay);
      await page.keyboard.press('Escape');
      await page.keyboard.press('ArrowDown');
      await page.keyboard.press('ArrowUp');
    }
  }
}

export function syntheticRemoteInteractionPoint(region, iframeBox) {
  const marker = region.coordinateSpace === 'remote-view-iframe'
    ? pixelMarkerClipForIframe(region, iframeBox)
    : region;
  return {
    x: marker.x + marker.width / 2,
    y: marker.y + marker.height / 2,
  };
}

async function waitForAuthoritativeHandoffResolution(resolutions, startIndex, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ready = resolutions.slice(startIndex).find(durableHandoffResolutionReady);
    if (ready) return ready;
    const terminal = resolutions.slice(startIndex).map(classifyHandoffResolutionFailure).find(Boolean);
    if (terminal) {
      const error = new Error(terminal.message);
      error.code = terminal.code;
      error.details = terminal.details;
      throw error;
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 200));
  }
  const observed = resolutions.slice(startIndex);
  const error = new Error('Authoritative ready handoff resolution was not observed before timeout');
  error.code = 'handoff_resolution_observation_timeout';
  error.details = {
    resolutionObservationCount: observed.length,
    resolutionStatuses: [...new Set(observed.map((item) => safeFailureToken(item?.status)).filter(Boolean))].sort(),
    resolutionReadinessGaps: [...new Set(observed.flatMap(handoffResolutionReadinessGaps))].sort(),
  };
  throw error;
}

export function handoffResolutionReadinessGaps(resolution) {
  const receipt = resolution?.presentationReceipt;
  return [
    [resolution?.resolved === true, 'resolved_false'],
    [resolution?.status === 'ready', 'status_not_ready'],
    [Number.isInteger(resolution?.presentationGeneration) && resolution.presentationGeneration > 0, 'presentation_generation_missing'],
    [receipt?.generation === resolution?.presentationGeneration, 'receipt_generation_mismatch'],
    [typeof receipt?.dashboardDeploymentGeneration === 'string' && receipt.dashboardDeploymentGeneration.length > 0, 'dashboard_generation_missing'],
    [receipt?.logicalBrowserId === resolution?.browserId, 'logical_browser_mismatch'],
    [Number.isInteger(receipt?.daemonOwnerGeneration) && receipt.daemonOwnerGeneration > 0, 'daemon_generation_missing'],
    [typeof receipt?.processInstanceDigest === 'string' && receipt.processInstanceDigest.length > 0, 'process_identity_missing'],
    [receipt?.targetId === resolution?.targetId, 'target_mismatch'],
    [receipt?.requiredStreamProvider === resolution?.viewStreamProvider, 'required_provider_mismatch'],
    [receipt?.observedStreamProvider === receipt?.requiredStreamProvider, 'observed_provider_mismatch'],
    [receipt?.state === 'ready', 'receipt_not_ready'],
  ].filter(([passed]) => !passed).map(([, code]) => code);
}

export function classifyHandoffResolutionFailure(resolution) {
  if (resolution?.status === 'failed' && resolution.failureCode) {
    const serviceStateLockTimeout = resolution.failureCode === 'service_state_lock_timeout';
    return {
      code: serviceStateLockTimeout ? 'service_state_lock_timeout' : 'handoff_resolution_failed',
      message: serviceStateLockTimeout
        ? 'Durable handoff resolution failed while waiting for Service State'
        : 'The durable handoff resolver returned a typed failure',
      details: {
        resolutionStatus: 'failed',
        failureCode: resolution.failureCode,
        ...(resolution.effectState ? { effectState: resolution.effectState } : {}),
        ...(resolution.retryDisposition ? { retryDisposition: resolution.retryDisposition } : {}),
        ...(Number.isSafeInteger(resolution.waitMs) ? { waitMs: resolution.waitMs } : {}),
      },
    };
  }
  if (resolution?.status === 'closed' && resolution.reopenRequired === true) {
    return {
      code: 'handoff_target_closed_operator_action_required',
      message: 'The durable handoff target is closed and requires explicit operator reopening',
      details: { resolutionStatus: 'closed', reopenRequired: true },
    };
  }
  if (resolution?.status && !['ready', 'converging'].includes(resolution.status)) {
    return {
      code: 'handoff_resolution_terminal',
      message: 'The durable handoff reached a terminal non-ready resolution',
      details: { resolutionStatus: String(resolution.status).slice(0, 64), reopenRequired: false },
    };
  }
  return null;
}

function durableHandoffResolutionReady(resolution) {
  const receipt = resolution?.presentationReceipt;
  return resolution?.resolved === true &&
    resolution.status === 'ready' &&
    Number.isInteger(resolution.presentationGeneration) &&
    resolution.presentationGeneration > 0 &&
    receipt?.generation === resolution.presentationGeneration &&
    typeof receipt.dashboardDeploymentGeneration === 'string' &&
    receipt.dashboardDeploymentGeneration.length > 0 &&
    receipt.logicalBrowserId === resolution.browserId &&
    Number.isInteger(receipt.daemonOwnerGeneration) &&
    receipt.daemonOwnerGeneration > 0 &&
    typeof receipt.processInstanceDigest === 'string' &&
    receipt.processInstanceDigest.length > 0 &&
    receipt.targetId === resolution.targetId &&
    receipt.requiredStreamProvider === resolution.viewStreamProvider &&
    receipt.observedStreamProvider === receipt.requiredStreamProvider &&
    receipt.state === 'ready';
}

function identityFromResolution(resolution, expected) {
  const tab = resolution.tab ?? {};
  const handle = tab.serviceTabHandle ?? {};
  const intent = resolution.open?.intent ?? {};
  const identity = {
    browserId: resolution.browserId ?? tab.browserId ?? handle.browserId ?? null,
    profileId: tab.profileId ?? tab.runtimeProfile ?? handle.profileId ?? intent.runtimeProfile ?? intent.profile ?? null,
    sessionId: resolution.sessionName ?? tab.sessionId ?? handle.sessionName ?? null,
    tabId: resolution.tabId ?? tab.tabId ?? tab.id ?? handle.tabId ?? null,
    targetId: resolution.targetId ?? tab.targetId ?? handle.targetId ?? null,
    visibleUrl: tab.url ?? resolution.visibleUrl ?? intent.url ?? null,
    pageMarker: tab.pageMarker ?? resolution.pageMarker ?? tab.title ?? null,
  };
  for (const field of REQUIRED_IDENTITY_FIELDS.filter((field) => !['pixelHash'].includes(field))) {
    if (identity[field] !== expected[field]) {
      const error = new Error(`Authoritative handoff resolution does not match expected ${field}`);
      error.code = 'visible_identity_mismatch';
      error.details = {
        identityField: field,
        expectedIdentityValueSha256: hashText(String(expected[field] ?? '')),
        observedIdentityValueSha256: hashText(String(identity[field] ?? '')),
        expectedTabMatchesTarget: canonicalTabIdentity(expected.tabId) === canonicalTabIdentity(expected.targetId),
        observedTabMatchesTarget: canonicalTabIdentity(identity.tabId) === canonicalTabIdentity(identity.targetId),
      };
      throw error;
    }
  }
  return identity;
}

function canonicalTabIdentity(value) {
  return String(value ?? '').replace(/^target:/, '');
}

function buildOracleSession({ clientId, handoff, resolvedAddresses, tls, initial, reconnect, reconnectObservations, expectedIdentity, urlEvidence, cookieMetadata, serverPhysicalBrowserLaunchDelta }) {
  const handoffId = handoff.pathname.split('/').filter(Boolean)[1];
  return {
    fixtureId: `live-${clientId}`,
    description: 'Live Plan 0158 external GitHub-hosted durable handoff observation.',
    initialHandoffId: handoffId,
    initialHandoffUrl: handoff.href,
    authenticated: true,
    externalVantage: {
      runnerId: clientId,
      outsideServiceHost: true,
      outsideServiceNetworkNamespace: true,
      publicEgressObserved: true,
    },
    operatorVisibleState: 'ready',
    readyObservedAt: initial.readyAt,
    firstUsablePixelsAt: initial.firstUsablePixelsAt,
    expectedIdentity,
    urlObservations: [
      { observationId: 'starting-handoff', role: 'starting_handoff', url: handoff.href, navigable: true, source: 'external_client' },
      ...urlEvidence.map((entry) => ({
        observationId: entry.evidenceId,
        role: entry.role,
        url: entry.url,
        navigable: true,
        source: entry.role === 'copied_action' ? 'clipboard' : entry.role === 'error_action' ? 'error_ui' : 'external_client',
      })),
    ],
    ingressChecks: [
      ingressCheck('dns', handoff.href, resolvedAddresses.length > 0, resolvedAddresses),
      ingressCheck('tls', handoff.href, tls.authorized === true),
      ingressCheck(
        'redirect',
        handoff.href,
        initial.navigation.status >= 200 && initial.navigation.status < 400,
        [],
        `redirect chain captured with ${initial.navigation.redirectCount} redirect responses`,
      ),
      ingressCheck('cookie', handoff.href, cookieMetadata.length > 0),
      ingressCheck('websocket', urlEvidence.find((item) => item.role === 'websocket_endpoint')?.url || handoff.href, urlEvidence.some((item) => item.role === 'websocket_endpoint')),
      ingressCheck('iframe', urlEvidence.find((item) => item.role === 'iframe_src')?.url || handoff.href, urlEvidence.some((item) => item.role === 'iframe_src')),
      ingressCheck(
        'form_action',
        urlEvidence.find((item) => item.role === 'form_action')?.url || handoff.href,
        initial.formActionCount > 0 || initial.formSurfaceAbsent === true,
        [],
        initial.formSurfaceAbsent ? 'DOM scan proved no form-action surface after authentication' : `DOM scan captured ${initial.formActionCount} form actions`,
      ),
      ingressCheck('reconnect', handoff.href, true),
    ],
    reconnects: reconnectObservations.map((observation, index) => ({
      reconnectId: `scheduled-reconnect-${index + 1}`,
      handoffId,
      handoffUrl: handoff.href,
      state: 'passed',
      operatorVisibleState: 'ready',
      readyObservedAt: observation.readyAt,
      firstUsablePixelsAt: observation.firstUsablePixelsAt,
      identity: observation.identity,
      physicalBrowserLaunchCount: index === reconnectObservations.length - 1
        ? serverPhysicalBrowserLaunchDelta
        : 0,
    })),
    captureGaps: [],
    expectedFindingCodes: [],
  };
}

function serviceBrowserObservationFromResolution(resolution, observedAt) {
  const receipt = resolution.presentationReceipt;
  const processIdentity = {
    browserId: resolution.browserId,
    logicalBrowserId: receipt.logicalBrowserId,
    daemonOwnerGeneration: receipt.daemonOwnerGeneration,
    processInstanceDigest: receipt.processInstanceDigest,
  };
  return {
    browserId: resolution.browserId,
    processIdentitySha256: canonicalHash(processIdentity),
    observedAt,
    source: 'authoritative_handoff_presentation_receipt',
  };
}

function ingressCheck(kind, targetUrl, passed, resolvedAddresses = [], passedDetail = null) {
  return {
    checkId: `live-${kind}`,
    kind,
    state: passed ? 'passed' : 'failed',
    targetUrl,
    resolvedAddresses,
    observedAt: new Date().toISOString(),
    detail: passed ? passedDetail : `${kind} evidence was not observed`,
  };
}

async function capturePerformance(page) {
  const session = await page.context().newCDPSession(page);
  await session.send('Performance.enable');
  const metrics = await session.send('Performance.getMetrics');
  return {
    capturedAt: new Date().toISOString(),
    metrics: Object.fromEntries(metrics.metrics.map((item) => [item.name, item.value])),
    navigation: await page.evaluate(() => performance.getEntriesByType('navigation').map((entry) => ({
      duration: entry.duration,
      domContentLoadedEventEnd: entry.domContentLoadedEventEnd,
      loadEventEnd: entry.loadEventEnd,
      transferSize: entry.transferSize,
    }))),
    resources: await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => ({
      initiatorType: entry.initiatorType,
      duration: entry.duration,
      transferSize: entry.transferSize,
    }))),
  };
}

async function resolvePublicAddresses(hostname) {
  const [v4, v6] = await Promise.all([
    dns.resolve4(hostname).catch(() => []),
    dns.resolve6(hostname).catch(() => []),
  ]);
  const addresses = [...new Set([...v4, ...v6])].sort();
  if (!addresses.length) throw new Error('Public DNS returned no addresses');
  if (addresses.some((address) => INTERNAL_HOST.test(address))) {
    throw new Error('Public DNS resolved to an internal address');
  }
  return addresses;
}

function observeTls(url) {
  return new Promise((resolvePromise, reject) => {
    const socket = tlsConnect({
      host: url.hostname,
      port: Number(url.port || 443),
      servername: url.hostname,
      rejectUnauthorized: true,
    });
    socket.setTimeout(15_000);
    socket.once('secureConnect', () => {
      const cert = socket.getPeerCertificate();
      const result = {
        authorized: socket.authorized,
        authorizationError: socket.authorizationError || null,
        protocol: socket.getProtocol(),
        cipher: socket.getCipher()?.name || null,
        peerFingerprint256: cert.fingerprint256 || null,
        serverName: url.hostname,
      };
      socket.end();
      resolvePromise(result);
    });
    socket.once('timeout', () => socket.destroy(new Error('TLS observation timed out')));
    socket.once('error', reject);
  });
}

function runnerEvidence(env) {
  const identity = {
    provider: 'github-actions',
    environment: env.RUNNER_ENVIRONMENT,
    os: env.RUNNER_OS,
    architecture: env.RUNNER_ARCH,
    runId: env.GITHUB_RUN_ID,
    runAttempt: env.GITHUB_RUN_ATTEMPT,
    job: env.GITHUB_JOB,
    runnerName: env.RUNNER_NAME,
  };
  return {
    provider: identity.provider,
    environment: identity.environment,
    os: identity.os,
    architecture: identity.architecture,
    runId: env.GITHUB_RUN_ID,
    runAttempt: env.GITHUB_RUN_ATTEMPT,
    job: env.GITHUB_JOB,
    runnerIdentitySha256: canonicalHash(identity),
  };
}

function distributedRunnerIdentity(env) {
  const runner = runnerEvidence(env);
  return {
    provider: 'github_actions',
    runnerId: runner.runnerIdentitySha256,
    runnerName: env.RUNNER_NAME,
    runnerOs: env.RUNNER_OS,
    runnerArch: env.RUNNER_ARCH,
  };
}

function globalCalibrationOrdinal(kind, localOrdinal, paceProfile) {
  const maximum = kind === 'dashboard_action' ? 25 : 5;
  if (!Number.isInteger(localOrdinal) || localOrdinal < 1 || localOrdinal > maximum) {
    throw new Error(`Invalid local ${kind} ordinal`);
  }
  return localOrdinal * 2 - (paceProfile === 'human_controller' ? 1 : 0);
}

function sanitizeUrlEvidence(entry) {
  return {
    evidenceId: entry.evidenceId,
    role: entry.role,
    url: redactOperatorUrl(entry.url),
    urlSha256: hashText(entry.url),
  };
}

function sanitizeIngressCheck(check) {
  return {
    ...check,
    targetUrl: redactOperatorUrl(check.targetUrl),
    targetUrlSha256: hashText(check.targetUrl),
  };
}

function assertSameIdentity(left, right, label) {
  for (const field of REQUIRED_IDENTITY_FIELDS) {
    if (left[field] !== right[field]) throw new Error(`${label} changed ${field}`);
  }
}

function assertSecretsAbsent(serialized, env) {
  for (const name of ['P158_DEV_HANDOFF_URL', 'P158_DEV_DASHBOARD_USERNAME', 'P158_DEV_DASHBOARD_PASSWORD']) {
    const value = env[name];
    if (value && serialized.includes(value)) throw new Error(`Receipt retained secret ${name}`);
  }
}

function parseSecretJson(value, label) {
  try {
    const parsed = JSON.parse(value);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error();
    return parsed;
  } catch {
    throw new Error(`Invalid ${label} JSON`);
  }
}

function hashText(value) {
  return createHash('sha256').update(String(value)).digest('hex');
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function artifactReceipts(root) {
  if (!existsSync(root)) return [];
  return listFiles(root)
    .filter((path) => !path.endsWith('receipt.json'))
    .map((path) => ({
      artifactId: basename(path),
      relativePath: path.slice(root.length + 1),
      bytes: statSync(path).size,
      sha256: sha256File(path),
    }))
    .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function requireCaptureArtifacts(artifacts) {
  for (const extension of ['.har', '.png', '.webm']) {
    if (!artifacts.some((artifact) => artifact.relativePath.endsWith(extension))) {
      throw new Error(`External evidence capture omitted required ${extension} artifact`);
    }
  }
  return artifacts;
}

function writeSanitizedHar(networkEntries, path) {
  const har = {
    log: {
      version: '1.2',
      creator: { name: 'agent-browser-p158-external-vantage', version: '1' },
      entries: networkEntries.map((entry) => ({
        startedDateTime: entry.timestamp,
        time: 0,
        request: {
          method: entry.method,
          url: entry.url,
          httpVersion: 'unknown',
          cookies: [],
          headers: [],
          queryString: [],
          headersSize: -1,
          bodySize: -1,
        },
        response: {
          status: entry.status,
          statusText: '',
          httpVersion: 'unknown',
          cookies: [],
          headers: [],
          content: { size: 0, mimeType: 'application/x-content-excluded-at-capture' },
          redirectURL: '',
          headersSize: -1,
          bodySize: -1,
        },
        cache: {},
        timings: { send: 0, wait: 0, receive: 0 },
        comment: `resourceType=${entry.resourceType}; urlSha256=${entry.urlSha256}`,
      })),
    },
  };
  writeFileSync(path, `${JSON.stringify(har, null, 2)}\n`, { mode: 0o600 });
}

function listFiles(root) {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const path = join(root, entry.name);
    return entry.isDirectory() ? listFiles(path) : [path];
  });
}

function safeErrorMessage(error, env) {
  let message = error instanceof Error ? error.message : String(error);
  for (const name of ['P158_DEV_HANDOFF_URL', 'P158_DEV_DASHBOARD_USERNAME', 'P158_DEV_DASHBOARD_PASSWORD']) {
    const value = env[name];
    if (value) message = message.split(value).join(`<redacted:${name}>`);
  }
  try {
    const handoff = new URL(env.P158_DEV_HANDOFF_URL);
    const handoffId = handoff.pathname.split('/').filter(Boolean)[1];
    if (handoffId) message = message.split(handoffId).join('<redacted-handoff-id>');
  } catch {
    // Invalid secret input is already represented by a typed failure receipt.
  }
  return message.slice(0, 1000);
}

export function externalVantageFailureRecord(error, env) {
  const typedCode = typeof error?.code === 'string' && /^[a-z0-9_]+$/.test(error.code)
    ? error.code
    : 'external_vantage_probe_failed';
  const details = safeExternalFailureDetails(error?.details);
  return {
    code: typedCode,
    message: safeErrorMessage(error, env),
    ...(details ? { details } : {}),
  };
}

function safeExternalFailureDetails(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const details = {};
  if (Number.isSafeInteger(value.iframeCount) && value.iframeCount >= 0) {
    details.iframeCount = value.iframeCount;
  }
  if (Array.isArray(value.iframePathClasses)) {
    details.iframePathClasses = value.iframePathClasses
      .filter((item) => item === 'guacamole' || item === 'unexpected')
      .slice(0, 10);
  }
  if (typeof value.observedPixelHash === 'string' && /^[a-f0-9]{64}$/.test(value.observedPixelHash)) {
    details.observedPixelHash = value.observedPixelHash;
  }
  for (const key of ['streamSignInExpired', 'cdpStreamConnecting']) {
    if (typeof value[key] === 'boolean') details[key] = value[key];
  }
  if (['closed', 'not_found', 'blocked', 'failed', 'unavailable'].includes(value.resolutionStatus)) {
    details.resolutionStatus = value.resolutionStatus;
  }
  if (typeof value.reopenRequired === 'boolean') details.reopenRequired = value.reopenRequired;
  for (const key of ['failureCode', 'effectState', 'retryDisposition']) {
    if (typeof value[key] === 'string' && /^[a-z0-9_]{1,64}$/.test(value[key])) {
      details[key] = value[key];
    }
  }
  if (Number.isSafeInteger(value.waitMs) && value.waitMs >= 0 && value.waitMs <= 300_000) {
    details.waitMs = value.waitMs;
  }
  if (Number.isSafeInteger(value.urlFindingCount) && value.urlFindingCount >= 0) {
    details.urlFindingCount = value.urlFindingCount;
  }
  if (Number.isSafeInteger(value.resolutionObservationCount) && value.resolutionObservationCount >= 0) {
    details.resolutionObservationCount = value.resolutionObservationCount;
  }
  for (const key of [
    'networkEntryCount',
    'guacamoleNetworkEntryCount',
    'websocketObservationCount',
    'consoleEntryCount',
  ]) {
    if (Number.isSafeInteger(value[key]) && value[key] >= 0) details[key] = value[key];
  }
  if (value.guacamoleHttpStatusCounts && typeof value.guacamoleHttpStatusCounts === 'object') {
    details.guacamoleHttpStatusCounts = Object.fromEntries(
      Object.entries(value.guacamoleHttpStatusCounts)
        .filter(([status, count]) => /^\d{3}$/.test(status) && Number.isSafeInteger(count) && count >= 0)
        .slice(0, 10),
    );
  }
  if (Array.isArray(value.resolutionStatuses)) {
    details.resolutionStatuses = value.resolutionStatuses
      .filter((item) => typeof item === 'string' && /^[a-z0-9_]{1,64}$/.test(item))
      .slice(0, 10);
  }
  if (Array.isArray(value.resolutionReadinessGaps)) {
    details.resolutionReadinessGaps = value.resolutionReadinessGaps
      .filter((item) => typeof item === 'string' && /^[a-z0-9_]{1,64}$/.test(item))
      .slice(0, 20);
  }
  if (typeof value.identityField === 'string' && REQUIRED_IDENTITY_FIELDS.includes(value.identityField)) {
    details.identityField = value.identityField;
  }
  for (const key of ['expectedIdentityValueSha256', 'observedIdentityValueSha256']) {
    if (typeof value[key] === 'string' && /^[a-f0-9]{64}$/.test(value[key])) {
      details[key] = value[key];
    }
  }
  for (const key of ['expectedTabMatchesTarget', 'observedTabMatchesTarget']) {
    if (typeof value[key] === 'boolean') details[key] = value[key];
  }
  if (Array.isArray(value.urlFindingRoles)) {
    details.urlFindingRoles = value.urlFindingRoles
      .filter((item) => EXTERNAL_URL_ROLES.includes(item))
      .slice(0, EXTERNAL_URL_ROLES.length);
  }
  if (Array.isArray(value.urlFindingCodes)) {
    details.urlFindingCodes = value.urlFindingCodes
      .filter((item) => EXTERNAL_HANDOFF_FINDING_CODES.includes(item))
      .slice(0, EXTERNAL_HANDOFF_FINDING_CODES.length);
  }
  return Object.keys(details).length > 0 ? details : null;
}

function safeFailureToken(value) {
  return typeof value === 'string' && /^[a-z0-9_]{1,64}$/.test(value) ? value : null;
}

function failureCalibrationTiming(env) {
  const startMs = Date.parse(env.P158_CALIBRATION_START_AT);
  if (!Number.isFinite(startMs)) return null;
  const observedMs = Date.now();
  return {
    calibrationStartAt: new Date(startMs).toISOString(),
    observedAt: new Date(observedMs).toISOString(),
    runnerStartDelayMs: observedMs - startMs,
  };
}

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
  }
  return value;
}

function withoutKey(value, omittedKey) {
  return Object.fromEntries(Object.entries(value).filter(([key]) => key !== omittedKey));
}

export async function aggregateExternalVantageDirectory(inputRoot, outputPath, runId, jobResults = {}) {
  const receipts = [];
  if (existsSync(inputRoot)) {
    for (const path of listFiles(inputRoot).filter((candidate) => /receipt\.json$/.test(candidate))) {
      receipts.push(JSON.parse(readFileSync(path, 'utf8')));
    }
  }
  let aggregate;
  try {
    aggregate = aggregateExternalVantageReceipts(receipts, { runId });
  } catch (error) {
    const diagnostic = {
      schemaVersion: EXTERNAL_VANTAGE_AGGREGATE_SCHEMA,
      planId: 'P158',
      runId,
      success: false,
      repairAttempted: false,
      retryCount: 0,
      jobResults,
      observedReceiptCount: receipts.length,
      observedClientIds: receipts.map((receipt) => receipt.clientId ?? null).sort(),
      failure: { code: 'external_vantage_aggregate_incomplete', message: safeErrorMessage(error, {}) },
    };
    aggregate = { ...diagnostic, aggregateSha256: canonicalHash(diagnostic) };
  }
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(aggregate, null, 2)}\n`, { mode: 0o600 });
  return aggregate;
}

async function main(args = process.argv.slice(2), env = process.env) {
  const command = args.shift();
  if (command === 'probe') {
    const clientId = takeOption(args, '--client-id');
    const paceProfile = takeOption(args, '--pace-profile');
    const mode = takeOption(args, '--mode');
    const w8ManifestPath = takeOptionalOption(args, '--w8-action-manifest');
    const outputDir = resolve(takeOption(args, '--output-dir'));
    if (args.length) throw new Error(`Unknown arguments: ${args.join(' ')}`);
    const w8ActionManifest = env.P158_W8_ACTION_MANIFEST_SHA256
      ? JSON.parse(readFileSync(resolve(w8ManifestPath), 'utf8'))
      : null;
    const receipt = await runExternalVantageProbe({
      env, clientId, paceProfile, mode, outputDir, w8ActionManifest,
    });
    process.stdout.write(`${JSON.stringify({ success: receipt.success, clientId, receiptSha256: canonicalHash(receipt) })}\n`);
    return;
  }
  if (command === 'aggregate') {
    const inputRoot = resolve(takeOption(args, '--input-root'));
    const outputPath = resolve(takeOption(args, '--output'));
    if (args.length) throw new Error(`Unknown arguments: ${args.join(' ')}`);
    const aggregate = await aggregateExternalVantageDirectory(
      inputRoot,
      outputPath,
      env.P158_RUN_ID,
      parseSecretJson(env.P158_EXTERNAL_JOB_RESULTS_JSON || '{}', 'external job results'),
    );
    process.stdout.write(`${JSON.stringify({ success: aggregate.success, output: basename(outputPath) })}\n`);
    if (!aggregate.success) process.exitCode = 1;
    return;
  }
  if (command === 'w8-actions') {
    const manifestPath = resolve(takeOption(args, '--manifest'));
    const aggregatePath = resolve(takeOption(args, '--external-aggregate'));
    const outputPath = resolve(takeOption(args, '--output'));
    if (args.length) throw new Error(`Unknown arguments: ${args.join(' ')}`);
    const result = executeP158W8ExternalActionManifest({
      manifest: JSON.parse(readFileSync(manifestPath, 'utf8')),
      externalVantageAggregate: JSON.parse(readFileSync(aggregatePath, 'utf8')),
      publicHandoffUrl: env.P158_DEV_HANDOFF_URL,
    });
    mkdirSync(dirname(outputPath), { recursive: true });
    const serialized = `${JSON.stringify(result, null, 2)}\n`;
    assertSecretsAbsent(serialized, env);
    writeFileSync(outputPath, serialized, { mode: 0o600 });
    process.stdout.write(`${JSON.stringify({ success: true, actionCount: result.actionCount, output: basename(outputPath) })}\n`);
    return;
  }
  throw new Error('Usage: run-p158-external-vantage.js probe|aggregate|w8-actions ...');
}

function takeOption(args, name) {
  const index = args.indexOf(name);
  if (index < 0 || index + 1 >= args.length) throw new Error(`Missing ${name}`);
  const [value] = args.splice(index + 1, 1);
  args.splice(index, 1);
  return value;
}

function takeOptionalOption(args, name) {
  const index = args.indexOf(name);
  if (index < 0) return null;
  if (index + 1 >= args.length) throw new Error(`Missing ${name}`);
  const [value] = args.splice(index + 1, 1);
  args.splice(index, 1);
  return value;
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  main().catch((error) => {
    process.stderr.write(`External vantage failed: ${safeErrorMessage(error, process.env)}\n`);
    process.exitCode = 1;
  });
}
